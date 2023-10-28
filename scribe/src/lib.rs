//mod scribe;
pub mod page;
pub mod bookcase;
pub mod secretary;


// Bookcase structure
pub const PAGE_SIZE: usize       = 4096 /*bytes*/;
pub const PAGE_COUNT: usize      = 512;
pub const PAGES_PER_WRITE: usize = 256;

// Page Structure
pub const DATA_SIZE: usize     = PAGE_SIZE - page::METADATA_SIZE /*bytes*/;
pub const WORDS: usize    = DATA_SIZE / 8;  /*u64s*/

pub type PageBytes = [u8; PAGE_SIZE];

pub mod memory_ops {

    pub fn to_byte_slice<'a, S, D>(obj: &S) -> &'a D
    where
        D: Sized,
        S: Sized 
    {
        unsafe {
            std::mem::transmute(obj)
        }
    }
    pub fn from_byte_slice<'a, T>(slice: &[u8]) -> Option<&T> {
        if slice.len() != std::mem::size_of::<T>() {
            return None;
        }
        let ptr = slice.as_ptr() as *const T;
        Some(unsafe {&*ptr })
    }
}

mod integration_tests {

    fn create_pages_from_queue() {
        use super::{
            WORDS,PAGES_PER_WRITE, PAGE_SIZE, PAGE_COUNT,
            memory_ops,
            secretary::scheduler::{
                WorkUnit,
                WorkQueueIterator,
                WorkUnitIterator,
            },
            page::Page,
            bookcase::BookCase
        };
        use std::{
            thread,
            sync::Arc,
            cell::Cell
        };
        use array_init::array_init;

        let mut handles = vec![];
        let q: WorkQueueIterator<2,1,1> = 0.into();
        let queue: Arc<WorkQueueIterator<2,1,1>> = Arc::new(q);

        for _ in 0..4 {
            let thread_queue = queue.clone();

            let handle = thread::spawn(move || {

                while let Some(range) = thread_queue.next() {
                    let mut write_buffer: [Page<WORDS>; PAGES_PER_WRITE] = [Page::default(); PAGES_PER_WRITE];
                    let mut wb_idx: usize = 0;

                    let start: WorkUnit = range.0;
                    let stop: WorkUnit = range.1;
                    let mut thread_work: WorkUnitIterator<2,1> = WorkUnitIterator::new(start, stop);

                    //TODO: Need to figure out how to track file change so that data can be written
                    //before changing over.
                    while let Some(work) = thread_work.next() {
                        let fid = work.0;
                        let pid = work.1;
                        let page: &mut Page<WORDS> = &mut write_buffer[wb_idx];
                        page.reinit(0xdead, fid, pid);
                        wb_idx += 1;
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
        
    }

}

