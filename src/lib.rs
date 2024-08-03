pub mod page;
pub mod chapter;
pub mod bookcase;

use std::sync::{Arc, atomic::AtomicU64};


pub const PAGE_BYTES: usize         = 4096 /*bytes*/;
pub const METADATA_BYTES: usize    = page::Page::<0>::METADATA_BYTES;
pub const DATA_BYTES: usize        = PAGE_BYTES - METADATA_BYTES;
pub const DATA_WORDS: usize        = DATA_BYTES / std::mem::size_of::<u64>();
pub const PAGE_COUNT: usize        = 512;
pub const PAGES_PER_CHAPTER: usize = 256;

pub type PageBytes = [u8; PAGE_BYTES];

/// Trait to define queue functionality that can dispatch
/// work consistently across thread boundaries.
pub trait WorkQueue: Sync + Send {
    fn take_work(&self) -> Option<(u64,u64)>;
    fn capacity(&self) -> u64;
    fn chunk_size(&self) -> u64;
}

pub trait AccessPattern: Sync + Send {

    /// Returns the lower bound of the access. In the case of sequential 
    /// access this is probably the same as the first element.
    fn lower_bound(&self) -> u64;

    /// This returns the upper bound of the access. In the case of a
    /// sequential access of stride one this is the last element to 
    /// be accessed
    fn upper_bound(&self) -> u64;

    /// The stride is how many elements are between the current access
    /// and the next. In the case of a sequential access pattern this
    /// is $1$.
    fn stride(&self) -> u64;
    
    /// This is the number of elements that should accessed at a given
    /// stride. In the case of a sequential access pattern this is
    /// $1$. 
    fn access_size(&self) -> u64;

    /// This is the number of accesses before this pattern should be 
    /// invalidated. In the case of a sequential access between $A$ 
    /// and $B$ this is $B-A$. This function is primarily intended
    /// for random access patterns in this there must be some way to
    /// indicate the access pattern has been exhausted-- a termination
    /// condition.
    fn access_count(&self) -> u64;
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
    fn upper_bound(&self)  -> u64 { self.0.upper_bound  }
    #[inline(always)]
    fn stride(&self)       -> u64 { self.0.stride       }
    #[inline(always)]
    fn access_size(&self) -> u64 { self.0.access_size   }
    #[inline(always)]
    fn access_count(&self) -> u64 { self.0.access_count }
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


