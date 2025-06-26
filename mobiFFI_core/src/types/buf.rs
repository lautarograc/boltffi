use core::mem::ManuallyDrop;
use std::alloc::{alloc, dealloc, Layout};
use core::ptr;

#[repr(C)]
pub struct FfiBuf<T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
}

impl<T> FfiBuf<T> {
    pub fn from_vec(vec: Vec<T>) -> Self {
        let mut vec = ManuallyDrop::new(vec);
        Self {
            ptr: vec.as_mut_ptr(),
            len: vec.len(),
            cap: vec.capacity(),
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        if self.ptr.is_null() {
            return Vec::new();
        }
        let vec = unsafe { Vec::from_raw_parts(self.ptr, self.len, self.cap) };
        core::mem::forget(self);
        vec
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr
    }
}

impl<T> Drop for FfiBuf<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.cap > 0 {
            unsafe {
                let _ = Vec::from_raw_parts(self.ptr, self.len, self.cap);
            }
        }
    }
}

impl<T> From<Vec<T>> for FfiBuf<T> {
    fn from(vec: Vec<T>) -> Self {
        Self::from_vec(vec)
    }
}
