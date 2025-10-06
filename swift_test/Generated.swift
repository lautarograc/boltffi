import Foundation

public struct DataPoint: Equatable {
    public var x: Double
    public var y: Double
    public var timestamp: Int64

    public init(x: Double, y: Double, timestamp: Int64) {
        self.x = x
        self.y = y
        self.timestamp = timestamp
    }
}

public struct SensorReading: Equatable {
    public var sensorId: Int32
    public var timestampMs: Int64
    public var value: Double

    public init(sensorId: Int32, timestampMs: Int64, value: Double) {
        self.sensorId = sensorId
        self.timestampMs = timestampMs
        self.value = value
    }
}

public enum Direction: Int32 {
    case north = 0
    case east = 1
    case south = 2
    case west = 3
}


public final class Counter {
    let handle: OpaquePointer

    init(handle: OpaquePointer) {
        self.handle = handle
    }

    public init() {
        let ptr = mffi_counter_new()
        self.init(handle: ptr)
    }

    deinit {
        mffi_counter_free(handle)
    }

    public func set(value: UInt64) {
        mffi_counter_set(handle, value)
    }

    public func increment() {
        mffi_counter_increment(handle)
    }

    public func get() -> UInt64 {
        return mffi_counter_get(handle)
    }
}


public final class Accumulator {
    let handle: OpaquePointer

    init(handle: OpaquePointer) {
        self.handle = handle
    }

    public init() {
        let ptr = mffi_accumulator_new()
        self.init(handle: ptr)
    }

    deinit {
        mffi_accumulator_free(handle)
    }

    public func add(amount: Int64) {
        mffi_accumulator_add(handle, amount)
    }

    public func get() -> Int64 {
        return mffi_accumulator_get(handle)
    }

    public func reset() {
        mffi_accumulator_reset(handle)
    }
}


public final class SensorMonitor {
    let handle: OpaquePointer

    init(handle: OpaquePointer) {
        self.handle = handle
    }

    public init() {
        let ptr = mffi_sensormonitor_new()
        self.init(handle: ptr)
    }

    deinit {
        mffi_sensormonitor_free(handle)
    }

    public func emitReading(sensorId: Int32, timestampMs: Int64, value: Double) {
        mffi_sensormonitor_emit_reading(handle, sensorId, timestampMs, value)
    }

    public func subscriberCount() -> UInt {
        return mffi_sensormonitor_subscriber_count(handle)
    }

    public func readings() -> AsyncStream<SensorReading> {
        AsyncStream<SensorReading> { continuation in
    let subscription = mffi_sensormonitor_readings(self.handle)
    
    Task {
        var buffer = [SensorReading](repeating: SensorReading(), count: 64)
        while true {
            let waitResult = mffi_sensormonitor_readings_wait(subscription, 100)
            if waitResult < 0 { break }
            
            let count = buffer.withUnsafeMutableBufferPointer { ptr in
                mffi_sensormonitor_readings_pop_batch(subscription, ptr.baseAddress, ptr.count)
            }
            
            for index in 0..<count {
                continuation.yield(buffer[index])
            }
        }
        
        mffi_sensormonitor_readings_free(subscription)
        continuation.finish()
    }
}
    }
}

