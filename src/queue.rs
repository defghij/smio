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

pub trait AccessPattern: Sync + Send {

    fn lower_bound(&self) -> u64;
    fn set_lower_bound(&self);

    fn upper_bound(&self) -> u64;
    fn set_upper_bound(&self);

    fn stride(&self) -> u64;
    fn set_stride(&self);
    
    fn access_size(&self) -> u64;
    fn set_access_size(&self);

    fn access_count(&self) -> u64;
    fn set_access_count(&self);
}

#[derive(Clone)]
pub struct AccessBounds {
    lower_bound: u64,
    upper_bound: u64,
    stride: u64,
    access_size: u64,
    access_count: u64,
}

/// This defines a monotonically increasing access pattern over a set of values.
#[derive(Clone)]
pub struct SerialAccess(AccessBounds);
impl SerialAccess {
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
    #[inline(always)]
    fn lower_bound(&self)  -> u64 { self.0.lower_bound  }
    #[inline(always)]
    fn set_lower_bound(&self) { self.0.lower_bound;  }

    #[inline(always)]
    fn upper_bound(&self)  -> u64 { self.0.upper_bound  }
    #[inline(always)]
    fn set_upper_bound(&self) { self.0.upper_bound;  }

    #[inline(always)]
    fn stride(&self)       -> u64 { self.0.stride       }
    #[inline(always)]
    fn set_stride(&self)       { self.0.stride;       }

    #[inline(always)]
    fn access_size(&self)  -> u64 { self.0.access_size  }
    #[inline(always)]
    fn set_access_size(&self)  { self.0.access_size;  }

    #[inline(always)]
    fn access_count(&self) -> u64 { self.0.access_count }
    #[inline(always)]
    fn set_access_count(&self) { self.0.access_count; }
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


