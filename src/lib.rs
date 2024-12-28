use std::{cell::Cell, sync::atomic::{AtomicU64, AtomicBool, AtomicUsize}};
use std::sync::atomic::Ordering;
use indicatif::HumanBytes;

pub mod page;
pub mod chapter;
pub mod bookcase;
pub mod queue;

/// Size of a Page in Bytes
pub const PAGE_BYTES: usize         = 4096 /*bytes*/;

/// Size of a Page's Metadata fields in bytes
pub const METADATA_BYTES: usize    = page::Page::<0>::METADATA_BYTES;

/// Size of a Page's Data Payload in bytes
pub const DATA_BYTES: usize        = PAGE_BYTES - METADATA_BYTES;
pub const DATA_WORDS: usize        = DATA_BYTES / std::mem::size_of::<u64>();
// Page contained in a Book (file).
pub const PAGE_COUNT: usize        = 512;

/// How many Pages are used in a single writable chunk
pub const PAGES_PER_CHAPTER: usize = 256;

pub type PageBytes = [u8; PAGE_BYTES];

// TODO: This needs to be redone. As it is, multiple threads
// can update within a sample period
  

struct Samples {
    samplers: usize,
    data: Vec<AtomicU64>
}
impl Samples {
    fn new(samplers: usize) -> Samples {
        Samples {
            samplers,
            data: (0..samplers + 1).map(|_| AtomicU64::new(0)).collect()
        }
    }

    fn get(&self, id: usize) -> Option<u64> {
        if id <= self.samplers {
            Some(self.data[id].load(Ordering::Relaxed))
        } else {
            None
        }
    }

    fn set(&self, id: usize, value: u64) -> Result<(), ()> {
        if id <= self.samplers {
            self.data[id].store(value, Ordering::Relaxed);
            Ok(())
        } else {
            Err(())
        }
    }

    #[allow(dead_code)]
    fn add(&self, id: usize, value: u64) -> Result<u64, ()> {
        if id <= self.samplers {
            Ok(self.data[id].fetch_add(value, Ordering::Relaxed))
        } else {
            Err(())
        }
    }

    fn swap(&self, id: usize, value: u64) -> Result<u64, ()> {
        if id <= self.samplers {
            Ok(self.data[id].swap(value, Ordering::Relaxed))
        } else {
            Err(())
        }
    }

    #[allow(dead_code)]
    fn modify<F>(&self, id: usize, operation: F) -> Result<(),()>
        where F: FnOnce(u64) -> u64

    {
        let current_value = self.get(id);
        if current_value.is_some() { 
            let new_value = operation(current_value.unwrap());
            self.set(id, new_value)
        } else {
            Err(())
        }
    }
}


thread_local! {
    static THREAD_ID: Cell<usize> = Cell::new(0);
}
static NEXT_THREAD_ID: AtomicUsize = AtomicUsize::new(1);

// TODO: There is an issue with this wherein the Inspector type isnt reporting all the
// data that gets written even though the sum of the work reported at the end of 
// each thread adds up to the expected number of bytes. 

pub struct Inspector {
    locked: AtomicBool,
    samples: Samples, 
    inspectors: usize,
} 
impl Inspector {

    pub fn new(thread_count: usize) -> Inspector {
        Inspector {
            locked: AtomicBool::new(false),
            samples: Samples::new(thread_count),
            inspectors: thread_count
        }
    }



    // Thread Management ------------------------------------------
    ///////////////////////////////////////////////////////////////
    
    pub fn register_thread(&self) {
        THREAD_ID.with(|thread_id| {
            if thread_id.get() == 0 {
                let new_tid = NEXT_THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                thread_id.set(new_tid);
            }
        });
    }

    #[inline(always)]
    fn thread_id(&self) -> usize {
        THREAD_ID.with(|thread_id| thread_id.get())
    }

    #[inline(always)]
    fn wait_for_unlock(&self) {
        while self.locked.load(Ordering::SeqCst) { std::hint::spin_loop() }
    }
    
    fn blocking<F>(&self, operation: F) -> Result<(), ()> 
        where F: FnOnce() -> Result<(), ()>
    {
        // Once we get past this spin we know self.updating := true and its from _our_ cmpxchg.
        // i.e. spin until we acquire lock.
        // TODO: Deadlock detection?
        while self.locked.compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire).is_err() {}

        operation()?;

        self.locked.store(false, Ordering::Relaxed); // Release lock
        Ok(())
    }

    fn nonblocking<F>(&self, operation: F) -> Result<(), ()> 
    where 
        F: FnOnce() -> Result<(), ()>
    {
        if self.locked.compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire).is_ok() {
            operation()?;
            self.locked.store(false, Ordering::Relaxed);
            Ok(())
        } else {
            Err(())
        }
    }
    

    // Read/Write Access ------------------------------------------
    ///////////////////////////////////////////////////////////////
    fn thread_counter_update(&self, value: u64) -> Result<(), ()> {
        let prev = self.samples.get(self.thread_id());
        if prev.is_some() {
            self.samples.set(self.thread_id(), value + prev.unwrap())
        } else {
            Err(())
        }
    }

    pub fn update_nonblocking(&self, value: u64) -> Result<(), ()> {
        self.nonblocking(|| { 
            self.thread_counter_update(value)
        })

    }

    pub fn update(&self, value: u64) -> Result<(), ()> {
        self.blocking(|| { 
            self.thread_counter_update(value)
        })

    }

    fn consolidate(&self) -> Result<(), ()> {
        let mut global_update: u64 = self.samples.get(self.inspectors).unwrap();
        for thread in 0..self.inspectors {
            global_update += self.samples.swap(thread, 0)?
        } 
        self.samples.set(self.inspectors, global_update)
    }

    pub fn flush_nonblocking(&self) -> Result<(), ()> {
        self.nonblocking(|| {
            self.consolidate()
        })
    }

    pub fn flush(&self) -> Result<(), ()> {
        self.blocking(|| {
            self.consolidate()
        })
    }



    // Reporting --------------------------------------------------
    ///////////////////////////////////////////////////////////////

    pub fn get_report_thread(&self) -> String {
        self.wait_for_unlock();
        let total: u64 = self.samples.get(self.thread_id()).unwrap_or(0);
        format!("{} total", total)
    }

    pub fn get_report_global(&self) -> String {
        self.wait_for_unlock();
        let _ = self.flush();
        let total: u64 = self.samples.get(self.inspectors).unwrap_or(0);
        format!("{} total", HumanBytes(total))
    }

    pub fn get_global_total(&self) -> u64 {
        self.wait_for_unlock();
        let _ = self.flush();
        self.samples.get(self.inspectors).unwrap_or(0)
    }
}
