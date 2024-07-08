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

#[cfg(test)]
mod integration_tests {

    /*
    use super::{
        PAGE_BYTES,
        bookcase::BookCase,
        chapter::Chapter,
        page::Page,
        PAGES_PER_CHAPTER
    };
    use std::fs::File;
    use std::io::{ Read, Write };
    use serial_test::serial;

    #[test]
    #[serial]
    fn single_threaded_test() {

        let pprefix: String = String::from("./testing");
        let dprefix: String = String::from("shelf");
        let fprefix: String = String::from("book");
        let dcount: u64 = 2;
        let fcount: u64 = 2;
        let pcount: u64 = 256;
        let seed: u64 =  0xD7D6D5D4D3D2D1D0;
        let mut bookcase: BookCase = BookCase::new(pprefix.to_owned(), 
                                                   dprefix.to_owned(),
                                                   dcount,
                                                   fprefix.to_owned(),
                                                   fcount,
                                                   PAGE_BYTES,
                                                   pcount);

        bookcase.construct().expect("Could not construct bookcase");

        const P: usize = PAGES_PER_CHAPTER;
        const W: usize = PAGE_BYTES / 8 - 4;
        const B: usize = Page::<W>::PAGE_BYTES * P;


        // Write to a File
        let mut chapter = Box::new(Chapter::<P,W,B>::new());
        (0..fcount).into_iter()
                   .for_each(|book| { 
                        let mut writable_book: File = bookcase.open_book(book, false, true).expect("Could  not open  file!");
                       
                        let full_writes: u64 = pcount / PAGES_PER_CHAPTER as u64;
                        let partial_writes: u64 = pcount % PAGES_PER_CHAPTER as u64;

                        //println!("Book {book} writes: full {full_writes}, partial {partial_writes}");

                        (0..full_writes).into_iter()
                                        .for_each(|fwrite|{
                                            let start: u64 = fwrite * PAGES_PER_CHAPTER as u64;
                                            let end: u64 = start + PAGES_PER_CHAPTER as u64 ;

                                            //println!("Full Write: ({start},{end}) @ file{book}");

                                            (start..end).for_each(|p|{
                                                chapter.mutable_page(p % PAGES_PER_CHAPTER as u64)
                                                       .reinit(seed, book, p, 0);
                                            });

                                            writable_book.write_all(chapter.bytes_all()).unwrap();
                                            writable_book.flush().expect("Could not flush file");

                                        });

                        chapter.zeroize();

                        if partial_writes > 0 {
                            let partial_start: u64 = full_writes * PAGES_PER_CHAPTER as u64;
                            let partial_end: u64 = partial_start + partial_writes;

                            //println!("Partial Write: ({partial_start},{partial_end}) @ file{book}");

                            (partial_start..partial_end).for_each(|p|{
                                chapter.mutable_page(p % partial_writes)
                                       .reinit(seed, book, p, 0);
                            });

                            let pages_to_write: usize = (partial_end - partial_start) as usize;
                            let partial_byte_count: usize = PAGE_BYTES * pages_to_write;


                            writable_book.write_all(chapter.bytes_upto(partial_byte_count)).unwrap();
                            writable_book.flush().expect("Could not flush file");

                            chapter.zeroize();
                        }
                        drop(writable_book);
                   });


        // Read from a File
        let mut chapter = Box::new(Chapter::<P,W,B>::new());
        (0..fcount).into_iter()
                   .for_each(|book| { 
                        let mut readable_book: File = bookcase.open_book(book, true, false).expect("Could  not open  file!");

                        loop {
                            let writable_buffer: &mut [u8] = chapter.mutable_bytes_all();
                            let bytes_read: usize = readable_book.read(writable_buffer).expect("Could not read from file!");

                            if bytes_read == 0 || bytes_read % PAGE_BYTES != 0 { break; }

                            chapter.pages_all()
                                   .iter()
                                   .for_each(|page|{
                                        if !page.is_valid() {
                                            let (s, f, p, m) = page.get_metadata();
                                            println!("Invalid Page Found: book {book}, page {page}");
                                            println!("Seed: 0x{s:X}\nFile: 0x{f:X}\nPage: 0x{p:X}\nMutations: 0x{m:X}");
                                            assert!(false);
                                        }
                                   });
                        }
                   });

        bookcase.demolish().expect("Could not demolish bookshelf");
    }
    */
}
