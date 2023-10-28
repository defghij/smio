

pub enum ExitCode {
    OK = 0,
    Critical = 1,
    MAJOR = 10,
    MINOR = 11,
    UNDEF
}

pub enum Verbosity {
    NONE,
    INFORMATIONAL,
    DEBUG,
    WARNING
}

use std::{
    io::Result,
    fs::File,
    os::fd::AsRawFd
};
use scribe::{
    PAGE_SIZE,
    PAGE_COUNT,
    PAGES_PER_WRITE,
    WORDS,
    page::Page,
    bookcase::{
        BookCase,
    },
    memory_ops::{
      to_byte_slice  
    }
};
use aio_rs::aio::{ 
    IoCmd,
    AioContext,
    AioRequest,
    aio_setup,
    aio_submit
};



fn main() -> Result<()> {

    /**************************
     * Set File Structure     *
     **************************/
    let path_prefix: String = String::from("/home/chuck/programming/testing");
    let directory_prefix: String = String::from("shelf");
    let file_prefix: String = String::from("book");
    let directory_count: u32 = 11;
    let file_count: u32 = 120;
    const WRITE_BUFFER_SIZE: usize = PAGES_PER_WRITE * PAGE_SIZE;
    let preseed: u32 = 0xdeadbeef;

    let bookcase: BookCase = BookCase::new(&path_prefix,
                                           &directory_prefix,
                                           directory_count,
                                           &file_prefix,
                                           file_count,
                                           PAGE_SIZE,
                                           PAGE_COUNT as u64);
    println!("About to build\n{bookcase}");
    bookcase.construct()?;
    println!("finished");
    let write_buffer: [u8; WRITE_BUFFER_SIZE] = [0; WRITE_BUFFER_SIZE];
    

        



    /**************************
     * Set Up IO              *
     **************************/
    let max_events: u32 = PAGES_PER_WRITE as u32;


    /**************************
     * Populate Books         *
     **************************/
    for bid in 0..bookcase.book_count() {
        let file: File = bookcase.open_book(bid, false, true);
        let file_descriptor = file.as_raw_fd();
        for pid in 0..(bookcase.page_count() as usize) {
            let page: Page<WORDS> = Page::new(preseed, bid as u32, pid as u64);
            let mut source_buffer: &[u8; PAGE_SIZE] = to_byte_slice(&page);
            let file_offset: isize = (PAGE_SIZE * pid) as isize;
            let request_tag: u64 = ((pid << 16) | bid as usize) as u64;
        }
    }

    //bookcase.demolish()?; // Revert directory structure. Shouldnt be used in practice.

    Ok(())
}
