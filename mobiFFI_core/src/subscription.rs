use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::ringbuffer::SpscRingBuffer;

pub struct EventSubscription<T: Send + 'static> {
    ring_buffer: SpscRingBuffer<T>,
    is_active: AtomicBool,
    notification_mutex: Mutex<()>,
    notification_condvar: Condvar,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    EventsAvailable = 1,
    Timeout = 0,
    Unsubscribed = -1,
}

impl<T: Send + 'static> EventSubscription<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            ring_buffer: SpscRingBuffer::new(capacity),
            is_active: AtomicBool::new(true),
            notification_mutex: Mutex::new(()),
            notification_condvar: Condvar::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Acquire)
    }

    pub fn push_event(&self, event: T) -> bool {
        if !self.is_active() {
            return false;
        }

        let push_succeeded = self.ring_buffer.push(event).is_ok();

        if push_succeeded {
            self.notification_condvar.notify_one();
        }

        push_succeeded
    }

    pub fn pop_event(&self) -> Option<T> {
        self.ring_buffer.pop()
    }

    pub fn pop_batch_into(&self, output_buffer: &mut [std::mem::MaybeUninit<T>]) -> usize {
        self.ring_buffer.pop_batch_into(output_buffer)
    }

    pub fn wait_for_events(&self, timeout_milliseconds: u32) -> WaitResult {
        if !self.is_active() {
            return WaitResult::Unsubscribed;
        }

        if self.ring_buffer.available_count() > 0 {
            return WaitResult::EventsAvailable;
        }

        let notification_guard = self.notification_mutex.lock().unwrap();
        let timeout_duration = Duration::from_millis(timeout_milliseconds as u64);

        let wait_result = self
            .notification_condvar
            .wait_timeout_while(notification_guard, timeout_duration, |_| {
                self.is_active() && self.ring_buffer.is_empty()
            });

        if !self.is_active() {
            return WaitResult::Unsubscribed;
        }

        match wait_result {
            Ok((_, timeout_result)) if timeout_result.timed_out() => WaitResult::Timeout,
            _ => {
                if self.ring_buffer.available_count() > 0 {
                    WaitResult::EventsAvailable
                } else {
                    WaitResult::Timeout
                }
            }
        }
    }

    pub fn unsubscribe(&self) {
        self.is_active.store(false, Ordering::Release);
        self.notification_condvar.notify_all();
    }

    pub fn available_count(&self) -> usize {
        self.ring_buffer.available_count()
    }
}

impl<T: Send + 'static> Drop for EventSubscription<T> {
    fn drop(&mut self) {
        self.unsubscribe();
    }
}

pub type SubscriptionHandle = *mut core::ffi::c_void;

pub fn subscription_new<T: Send + 'static>(capacity: usize) -> SubscriptionHandle {
    let subscription = Box::new(EventSubscription::<T>::new(capacity));
    Box::into_raw(subscription) as SubscriptionHandle
}

pub unsafe fn subscription_push<T: Send + 'static>(
    handle: SubscriptionHandle,
    event: T,
) -> bool {
    if handle.is_null() {
        return false;
    }
    let subscription = unsafe { &*(handle as *const EventSubscription<T>) };
    subscription.push_event(event)
}

pub unsafe fn subscription_pop_batch<T: Send + Copy + 'static>(
    handle: SubscriptionHandle,
    output_ptr: *mut T,
    output_capacity: usize,
) -> usize {
    if handle.is_null() || output_ptr.is_null() || output_capacity == 0 {
        return 0;
    }

    let subscription = unsafe { &*(handle as *const EventSubscription<T>) };
    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(
            output_ptr as *mut std::mem::MaybeUninit<T>,
            output_capacity,
        )
    };

    subscription.pop_batch_into(output_slice)
}

pub unsafe fn subscription_wait<T: Send + 'static>(
    handle: SubscriptionHandle,
    timeout_milliseconds: u32,
) -> i32 {
    if handle.is_null() {
        return WaitResult::Unsubscribed as i32;
    }

    let subscription = unsafe { &*(handle as *const EventSubscription<T>) };
    subscription.wait_for_events(timeout_milliseconds) as i32
}

pub unsafe fn subscription_unsubscribe<T: Send + 'static>(handle: SubscriptionHandle) {
    if handle.is_null() {
        return;
    }

    let subscription = unsafe { &*(handle as *const EventSubscription<T>) };
    subscription.unsubscribe();
}

pub unsafe fn subscription_free<T: Send + 'static>(handle: SubscriptionHandle) {
    if handle.is_null() {
        return;
    }

    let subscription = unsafe { Box::from_raw(handle as *mut EventSubscription<T>) };
    drop(subscription);
}

struct SubscriberSlot<T: Send + 'static> {
    subscription_ptr: AtomicPtr<EventSubscription<T>>,
}

impl<T: Send + 'static> SubscriberSlot<T> {
    const fn empty() -> Self {
        Self {
            subscription_ptr: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    fn try_claim(&self, subscription: &Arc<EventSubscription<T>>) -> bool {
        let raw_ptr = Arc::as_ptr(subscription) as *mut EventSubscription<T>;
        self.subscription_ptr
            .compare_exchange(
                std::ptr::null_mut(),
                raw_ptr,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn load_subscription(&self) -> Option<*const EventSubscription<T>> {
        let ptr = self.subscription_ptr.load(Ordering::Acquire);
        (!ptr.is_null()).then_some(ptr)
    }

    fn clear_if_inactive(&self) {
        let ptr = self.subscription_ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            return;
        }

        let is_active = unsafe { (*ptr).is_active() };
        if !is_active {
            self.subscription_ptr
                .compare_exchange(ptr, std::ptr::null_mut(), Ordering::AcqRel, Ordering::Acquire)
                .ok();
        }
    }

    fn is_occupied(&self) -> bool {
        let ptr = self.subscription_ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            return false;
        }
        unsafe { (*ptr).is_active() }
    }
}

pub struct StreamProducer<T: Send + Copy + 'static, const MAX_SUBSCRIBERS: usize = 32> {
    subscriber_slots: [SubscriberSlot<T>; MAX_SUBSCRIBERS],
    default_capacity: usize,
}

impl<T: Send + Copy + 'static, const MAX_SUBSCRIBERS: usize> StreamProducer<T, MAX_SUBSCRIBERS> {
    pub fn new(default_capacity: usize) -> Self {
        Self {
            subscriber_slots: core::array::from_fn(|_| SubscriberSlot::empty()),
            default_capacity,
        }
    }

    pub fn subscribe(&self) -> Arc<EventSubscription<T>> {
        self.subscribe_with_capacity(self.default_capacity)
    }

    pub fn subscribe_with_capacity(&self, capacity: usize) -> Arc<EventSubscription<T>> {
        let subscription = Arc::new(EventSubscription::new(capacity));

        self.subscriber_slots.iter().for_each(|slot| slot.clear_if_inactive());

        let slot_claimed = self
            .subscriber_slots
            .iter()
            .any(|slot| slot.try_claim(&subscription));

        if !slot_claimed {
            eprintln!("StreamProducer: all {} subscriber slots full", MAX_SUBSCRIBERS);
        }

        subscription
    }

    pub fn push(&self, event: T) {
        self.subscriber_slots.iter().for_each(|slot| {
            if let Some(subscription_ptr) = slot.load_subscription() {
                let subscription = unsafe { &*subscription_ptr };
                if subscription.is_active() {
                    subscription.push_event(event);
                }
            }
        });
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscriber_slots
            .iter()
            .filter(|slot| slot.is_occupied())
            .count()
    }
}

impl<T: Send + Copy + 'static, const MAX_SUBSCRIBERS: usize> Default for StreamProducer<T, MAX_SUBSCRIBERS> {
    fn default() -> Self {
        Self::new(256)
    }
}

unsafe impl<T: Send + Copy + 'static, const MAX_SUBSCRIBERS: usize> Send for StreamProducer<T, MAX_SUBSCRIBERS> {}
unsafe impl<T: Send + Copy + 'static, const MAX_SUBSCRIBERS: usize> Sync for StreamProducer<T, MAX_SUBSCRIBERS> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_subscription_push_pop() {
        let subscription = EventSubscription::<i32>::new(16);
        assert!(subscription.push_event(42));
        assert!(subscription.push_event(100));
        assert_eq!(subscription.pop_event(), Some(42));
        assert_eq!(subscription.pop_event(), Some(100));
        assert_eq!(subscription.pop_event(), None);
    }

    #[test]
    fn test_subscription_unsubscribe_stops_push() {
        let subscription = EventSubscription::<i32>::new(16);
        assert!(subscription.push_event(1));
        subscription.unsubscribe();
        assert!(!subscription.push_event(2));
        assert!(!subscription.is_active());
    }

    #[test]
    fn test_subscription_wait_immediate_return() {
        let subscription = EventSubscription::<i32>::new(16);
        subscription.push_event(42);
        assert_eq!(
            subscription.wait_for_events(1000),
            WaitResult::EventsAvailable
        );
    }

    #[test]
    fn test_subscription_wait_timeout() {
        let subscription = EventSubscription::<i32>::new(16);
        assert_eq!(subscription.wait_for_events(10), WaitResult::Timeout);
    }

    #[test]
    fn test_subscription_cross_thread() {
        use std::sync::Arc;

        let subscription = Arc::new(EventSubscription::<i32>::new(1024));
        let producer_subscription = Arc::clone(&subscription);

        let producer_thread = thread::spawn(move || {
            (0..100).for_each(|index| {
                producer_subscription.push_event(index);
                thread::sleep(Duration::from_micros(100));
            });
        });

        let mut received_events = Vec::new();
        while received_events.len() < 100 {
            let wait_result = subscription.wait_for_events(100);
            if wait_result == WaitResult::Unsubscribed {
                break;
            }

            while let Some(event) = subscription.pop_event() {
                received_events.push(event);
            }
        }

        producer_thread.join().unwrap();
        assert_eq!(received_events.len(), 100);
        assert!(received_events.iter().enumerate().all(|(index, &value)| value == index as i32));
    }
}
