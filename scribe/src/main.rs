

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
    fs::File
};
use scribe::{
    page::Page,
    scribe::{
        BookCase,
        PAGE_SIZE,
        DATA_SIZE,
        PAGE_COUNT
    }
};
use aio_rs::aio::{ 
    AioContext,
    aio_setup
};



fn main() -> Result<()> {

    /**************************
     * Set File Structure     *
     **************************/
    let path_prefix: String = String::from("/home/chuck/programming/testing");
    let directory_prefix: String = String::from("shelf");
    let file_prefix: String = String::from("book");
    let directory_count: u32 = 8;
    let file_count: u32 = 120;
    let preseed: u32 = 0xdeadbeef;

    let bookcase: BookCase = BookCase::new(&path_prefix,
                                           &directory_prefix,
                                           directory_count,
                                           &file_prefix,
                                           file_count,
                                           PAGE_SIZE,
                                           PAGE_COUNT as u64);
    println!("About to build\n{bookcase}");
    bookcase.build()?;
    println!("finished");



    /**************************
     * Set Up IO              *
     **************************/
    let max_events: u32 = 1;
    let mut ctx: AioContext = AioContext::new();
    let ret = aio_setup(max_events, &mut ctx);
    
    if ret.is_err() { panic!("Failed with error: {}", ret.unwrap()); }
    /* // Setup Request.
    /// let file_descriptor = tmpfile.into_raw_fd();
    /// let file_offset: isize = 0;
    /// let request_tag: u64 = 0xAAAA;
    /// let request_code: IoCmd = IoCmd::Pread; 
    /// let mut destination_buffer: [u8; READ_SIZE] = [0; READ_SIZE];
    ///
    /// let iocb = AioRequest::new().add_fd(file_descriptor)
    ///                       .add_offset(file_offset)
    ///                       .add_tag(request_tag)
    ///                       .add_opcode(request_code)
    ///                       .add_buffer(&mut destination_buffer);
    /// let mut iocbs: [AioRequest; 1] = [iocb];
    ///
    /// // Submit I/O requests.
    /// let ret = aio_submit(ctx, &mut iocbs);
    /// if ret.is_err() { panic!("Failed to submit 2 iocbs: {}", ret.unwrap_err()); } 
    ///
    /// # let submitted = ret.unwrap();
    /// # assert!(submitted == 1, "Failed to submit iocb!");
    */


    /**************************
     * Populate Books         *
     **************************/
    for book in 0..bookcase.book_count() {
        let file: File = bookcase.open_book(book, false, true);
        for page in 0..(bookcase.page_count() as usize) {
            let page: Page<DATA_SIZE> = Page::new(book as u32, page as u64, preseed);
        }
    }

    bookcase.demolish()?; // Revert directory structure. Shouldnt be used in practice.

    Ok(())
}
