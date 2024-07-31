pub mod page;
pub mod chapter;
pub mod bookcase;



pub const PAGE_BYTES: usize         = 4096 /*bytes*/;
pub const METADATA_BYTES: usize    = page::Page::<0>::METADATA_BYTES;
pub const DATA_BYTES: usize        = PAGE_BYTES - METADATA_BYTES;
pub const DATA_WORDS: usize        = DATA_BYTES / std::mem::size_of::<u64>();
pub const PAGE_COUNT: usize        = 512;
pub const PAGES_PER_CHAPTER: usize = 256;

pub type PageBytes = [u8; PAGE_BYTES];



use std::sync::{Arc, atomic::AtomicU64};
#[derive(Clone)]
pub struct WorkQueue { 
    current: Arc<AtomicU64>,
    pub capacity: u64,
    pub window: u64,
    pub step: u64
}
impl WorkQueue {
    pub fn new(capacity: u64, step: u64, window: u64) -> WorkQueue {
        let current: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        WorkQueue {
            current,
            capacity,
            window,
            step
        }
    }
    pub fn take_work(&self) -> Option<(u64, u64)> {
        let work = self.current.fetch_add(self.step, std::sync::atomic::Ordering::Relaxed);

        let x: u64 = work % self.window;
        let y: u64 = work / self.window;
        Some((x, y))
    }

    pub fn update_capacity(&mut self, capacity: u64) {
        self.capacity = capacity;
    }
}
