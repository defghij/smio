use std::sync::{atomic::{AtomicU64, Ordering}, Arc};

pub struct Bounds {
    pub lower: u64,
    pub upper: u64
}




#[derive(Clone)]
pub struct Queue {
    /// A closure or function that defines how to calculate the
    /// next `current` value.
    /// TODO: Provide a default serial monotonic implementation
    next: Arc<dyn Fn(u64) -> Option<u64> + Send+Sync+'static>,

    /// `current` is the current counter value for the queue.
    /// This value should be bounded by [lower_bound,upper_bound).
    /// where the lower bound is inclusive and the upper bound is 
    /// exclusive.
    current: Arc<AtomicU64>,

    // The start (inclusive) of the work range which queue
    // will allow as valid
    lower_bound: u64,

    // The end (exclusive) of the work range which queue
    // will allow as valid
    upper_bound: u64,
} impl Queue {
    pub fn new<F>(lower_bound: u64, upper_bound: u64, next: F) -> Self 
    where F: Fn(u64) -> Option<u64> + Send+Sync+'static {
        Self {
            next: Arc::new(next),
            current: Arc::new(AtomicU64::new(lower_bound)),
            lower_bound,
            upper_bound,
        }
    }

    pub fn take_work(&self) -> Option<u64> {
        loop {
            // Figure out if the next `current` is value given the bounds
            let current = self.current.load(Ordering::Relaxed);
            let next = (self.next)(current)?;

            if next <= self.lower_bound || self.upper_bound <= next {
                return None;
            }

            // Attempt to set the new `current` value. If fail, loop and try again.
            if self.current
                   .compare_exchange(current, next, Ordering::SeqCst, Ordering::Relaxed)
                   .is_ok() 
            {
                return Some(current);
            }
        }
    }
}


