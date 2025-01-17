use std::sync::{atomic::AtomicU64, Arc};


#[derive(Clone)]
pub struct SimpleQueue { 
    current: Arc<AtomicU64>,
    pub capacity: u64,
    pub window: u64,
    pub step: u64
}
impl SimpleQueue {
    pub fn new(capacity: u64, step: u64, window: u64) -> SimpleQueue {
        let current: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        SimpleQueue {
            current,
            capacity,
            window,
            step
        }
    }

    pub fn take_work(&self) -> Option<(u64, u64)> {
        // Use Ordering::Relaxed because we dont actually care about the order threads
        // pull work from the queue-- only that the value is consistent across threads.
        // That is we require atomic updates ony.
        let work = self.current.fetch_add(self.step, std::sync::atomic::Ordering::Relaxed);

        let x: u64 = work % self.window;
        let y: u64 = work / self.window;
        Some((x, y))
    }

    pub fn update_capacity(&mut self, capacity: u64) {
        self.capacity = capacity;
    }
     
    pub fn reset(&self) {
        self.current.store(0,std::sync::atomic::Ordering::Release);
    }
}

/// Trait to define queue functionality that can dispatch
/// work consistently across thread boundaries.
pub trait WorkQueue: Sync + Send {
    fn take_work(&self) -> Option<(u64,u64)>;
    fn capacity(&self) -> u64;
    fn chunk_size(&self) -> u64;
}

// TODO This should really have a `next` function that takes a closure
// and yields the next element element (u64)
pub trait AccessPattern: Sync + Send {

    fn lower_bound(&self) -> u64;
    fn set_lower_bound(&self);

    fn upper_bound(&self) -> u64;
    fn set_upper_bound(&self);

    fn stride(&self) -> u64;
    fn set_stride(&selfi stride: u64);
    
    fn access_size(&self) -> u64;
    fn set_access_size(&self, size: u64);

    fn access_count(&self) -> u64;
    fn set_access_count(&self, count: u64);
}

#[derive(Clone)]
pub struct AccessBounds {
    lower_bound: u64,
    upper_bound: u64,
    stride: u64,
    access_size: u64,
    access_count: u64,
}

/// TODO: This should probaby be rolled into Queue.
/// This defines a monotonically increasing access pattern over a set of values.
#[derive(Clone)]
pub struct SerialAccess(AccessBounds);
impl SerialAccess {
    /// Creates a new access pattern. 
    /// Note that lower and upper bound assume zero-based indexing. `access_size` is the number
    /// of items each access will pull. This is the size of the work chunk. Returns a 
    /// `SerialAccess` which implements the `AccessPattern` trait for use with
    /// `Queue<AccessPattern>`.
    /// 
    /// # Example
    ///
    /// ```
    /// // Will create a 5 element access pattern that will denote work in the following way
    /// // [0,1],[2.3],[4,5],[6,7],[8,9]
    /// let serial: SerialAccess = SerialAccess::new(0, 9, 1, 2);
    /// ```

    pub fn new(lower_bound: u64, upper_bound: u64, stride: u64, access_size: u64) -> SerialAccess {
        let access_count: u64 = (upper_bound - lower_bound) / stride;
        SerialAccess(
           AccessBounds {
               lower_bound,
               upper_bound,
               stride,
               access_size,
               access_count
           }
        )
    }
} impl AccessPattern for SerialAccess {
    fn lower_bound(&self) -> u64 { self.0.lower_bound  }
    fn set_lower_bound(&self)    { self.0.lower_bound; }

    fn upper_bound(&self) -> u64 { self.0.upper_bound  }
    fn set_upper_bound(&self)    { self.0.upper_bound; }

    fn stride(&self) -> u64 { self.0.stride  }
    fn set_stride(&self)    { self.0.stride; }

    fn access_size(&self) -> u64 { self.0.access_size  }
    fn set_access_size(&self)    { self.0.access_size; }

    fn access_count(&self) -> u64 { self.0.access_count }
    fn set_access_count(&self)    { self.0.access_count;}
}


#[derive(Clone)]
pub struct Queue<A: AccessPattern> { 
    access_pattern: A,
    accesses: u64,
    window: u64,
    current: Arc<AtomicU64>,
} impl<A: AccessPattern> Queue<A> {
    pub fn new(window: u64, access_pattern: A) -> Queue<A> {
        let current: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        Queue {
            access_pattern,
            accesses: 0,
            window,
            current,
        }
    }
} impl<A: AccessPattern> WorkQueue for Queue<A> {
    fn take_work(&self) -> Option<(u64, u64)> {
        if self.accesses > self.access_pattern.access_count() {
            return None
        }
        // Ordering: Use Ordering::Relaxed because we dont actually care about the order threads
        // pull work from the queue-- only that the value is consistent across threads.
        // That is we require atomic updates ony.
        //
        // Serial: This is a serial access  
        let next_value: u64 = self.access_pattern.stride() + self.access_pattern.access_size() - 1;
        let work = self.current.fetch_add(next_value, std::sync::atomic::Ordering::Relaxed);

        if self.access_pattern.lower_bound() <= work && work <= self.access_pattern.upper_bound() { 
            let x: u64 = work % self.window;
            let y: u64 = work / self.window;

            Some((x, y))
        } else {
            None
        }
    }

    #[inline(always)]
    fn capacity(&self) -> u64 {
        self.access_pattern.upper_bound() - self.access_pattern.lower_bound()
    }

    #[inline(always)]
    fn chunk_size(&self) -> u64 {
        self.access_pattern.access_size()
    }
}


