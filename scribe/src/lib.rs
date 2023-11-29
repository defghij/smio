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
            println!("Memory sizes incompatible: {} != {}", slice.len(), std::mem::size_of::<T>());
            return None;
        }
        let ptr = slice.as_ptr() as *const T;
        Some(unsafe {&*ptr })
    }
}

#[cfg(test)]
mod integration_tests {
    use super::{
        WORDS,PAGES_PER_WRITE, PAGE_SIZE, PAGE_COUNT,
        //memory_ops,
        secretary::scheduler::{
            WorkUnit,
            WorkQueueIterator,
            WorkUnitIterator,
        },
        page::Page,
        bookcase::BookCase
    };
    use std::{
        fs::File,
        io:: {Write,Read},
        //thread,
        sync::Arc,
    };

    const DIRECTORY_COUNT: usize = 2;
    const FILE_COUNT: usize = 2;
    type WorkerQueue = Arc<WorkQueueIterator<PAGE_COUNT,FILE_COUNT,PAGES_PER_WRITE>>;
    type WorkIterator = WorkUnitIterator<PAGE_COUNT,FILE_COUNT>;

    fn do_write(buffer: &Vec<Page>, file: &mut File) {
        let write_buffer: &[u8; PAGE_COUNT * PAGE_SIZE] = super::memory_ops::to_byte_slice(buffer);
        file.write(write_buffer).unwrap();
    }

    fn do_read<'a>(buffer: &'a mut Vec<u8>, file: &mut File) -> &'a Vec<Page> {
        
        let _ = file.read_exact(vec![0; PAGES_PER_WRITE * PAGE_SIZE].as_mut_slice());
        let page_buffer: &Vec<Page> = super::memory_ops::from_byte_slice(buffer).expect("Could not transmute page!");
        page_buffer
    }

    fn thread_write(queue: WorkerQueue, bookcase: Arc<BookCase>) {
        while let Some(range) = queue.next() {
            let mut page_buffer: Vec<Page> = vec![Page::default(); PAGES_PER_WRITE];
            let mut wb_idx: usize = 0;

            let start: WorkUnit = range.0;
            let stop: WorkUnit = range.1;

            let mut fid_active: u64 = start.0.0;
            let mut file_active: File = bookcase.open_book(fid_active, false, true);


            let mut thread_work: WorkIterator = WorkUnitIterator::new(start, stop);

            while let Some(work) = thread_work.next() {   // We have one (file,page) and
                let fid = work.0;                         // create the associated page in the
                let pid = work.1;                         // write buffer.
                
                if fid_active != fid {
                    // Complete any outstanding writes for this file
                    do_write(&page_buffer, &mut file_active);

                    // Open new file for writing.
                    fid_active = fid;
                    file_active = bookcase.open_book(fid_active, false, true);
                }

                let page: &mut Page = &mut page_buffer[wb_idx];
                page.reinit(0xdead, fid as u64, pid as u64, 0);
                wb_idx += 1;

                if wb_idx == page_buffer.len() {
                    do_write(&page_buffer, &mut file_active);
                }
            }
        }
    }

    fn data_verify(queue: WorkerQueue, bookcase: Arc<BookCase>) {
        while let Some(range) = queue.next() {
            let mut page_buffer: Vec<Page> = vec![Page::default(); PAGES_PER_WRITE];
            let mut read_buffer: Vec<u8> = vec![0; PAGES_PER_WRITE * PAGE_SIZE]; 
            let mut wb_idx: usize = 0;

            let start: WorkUnit = range.0;
            let stop: WorkUnit = range.1;

            let mut fid_active: u64 = start.0.0;
            let mut file_active: File = bookcase.open_book(fid_active, false, true);


            let mut thread_work: WorkIterator = WorkUnitIterator::new(start, stop);

            while let Some(work) = thread_work.next() {   // We have one (file,page) and
                let fid = work.0;                         // create the associated page in the
                let pid = work.1;                         // write buffer.
                
                if fid_active != fid {
                    // Complete any outstanding writes for this file
                    let page_buffer: &Vec<Page> = do_read(&mut read_buffer, &mut file_active);
                    for page in page_buffer.iter() {
                        assert!(page.validate_page_with(0xdead, fid as u64, pid as u64, 0));
                    }

                    // Open new file for writing.
                    fid_active = fid;
                    file_active = bookcase.open_book(fid_active, true, false);
                }

                let page: &mut Page = &mut page_buffer[wb_idx];
                page.reinit(0xdead, fid as u64, pid as u64, 0);
                wb_idx += 1;

                if wb_idx == page_buffer.len() {
                    let page_buffer: &Vec<Page> = do_read(&mut read_buffer, &mut file_active);
                    for page in page_buffer.iter() {
                        assert!(page.validate_page_with(0xdead, fid as u64, pid as u64, 0));
                    }
                }
            }
        }
    }

    #[test]
    fn create_pages_from_queue() {

        let pprefix: String = String::from("/home/chuck/programming/testing");
        let dprefix: String = String::from("shelf");
        let fprefix: String = String::from("book");

        let bookcase: Arc<BookCase> = Arc::new(
                BookCase::new(pprefix.to_owned(),
                              dprefix.to_owned(),
                              DIRECTORY_COUNT as u64,
                              fprefix.to_owned(),
                              FILE_COUNT as u64,
                              PAGE_SIZE,
                              PAGE_COUNT as u64)
                );
        bookcase.construct().expect("Could not create test bookcase structures.");

        thread_write(Arc::new(0.into()), bookcase.clone());
        //data_verify(Arc::new(0.into()), bookcase.clone());

    
        //bookcase.demolish().expect("Could not demolish test bookcase");
        assert!(true);
    }
}
