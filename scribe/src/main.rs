

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

<<<<<<< Updated upstream
use std::{
    io::Result,
    fs::File,
    os::fd::AsRawFd
};
use scribe::{
    page::Page,
    scribe::{
        BookCase,
        PAGE_SIZE,
        WORDS,
        PAGE_COUNT,
        PAGES_PER_WRITE
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
=======
use std::io::Result;

    use scribe::{
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
        fs::File,
        io:: {Write,Read},
        thread,
        sync::Arc,
    };

    const DIRECTORY_COUNT: usize = 2;
    const FILE_COUNT: usize = 2;
    type WorkerQueue = Arc<WorkQueueIterator<PAGE_COUNT,FILE_COUNT,PAGES_PER_WRITE>>;
    type WorkIterator = WorkUnitIterator<PAGE_COUNT,FILE_COUNT>;

fn do_write<const N: usize>(buffer: &[Page<N>; PAGES_PER_WRITE], file: &mut File) {
    let write_buffer: &[u8; PAGE_COUNT * PAGE_SIZE] = memory_ops::to_byte_slice(buffer);
    file.write(write_buffer).unwrap();
}
>>>>>>> Stashed changes

fn do_read<'a, const N: usize>(buffer: &'a mut [u8; PAGES_PER_WRITE * PAGE_SIZE], file: &mut File) -> &'a [Page<WORDS>; PAGES_PER_WRITE] {
    let _ = file.read_exact(buffer);
    let page_buffer: &[Page<WORDS>; PAGES_PER_WRITE] = memory_ops::from_byte_slice(buffer).expect("Could not transmute page!");
    page_buffer
}

fn thread_write(queue: WorkerQueue, bookcase: Arc<BookCase>) {
    while let Some(range) = queue.next() {
        let mut page_buffer: Box<[Page<WORDS>; PAGES_PER_WRITE]> = Box::new([Page::default(); PAGES_PER_WRITE]);
        let mut wb_idx: usize = 0;

        let start: WorkUnit = range.0;
        let stop: WorkUnit = range.1;

        let mut fid_active: u32 = start.0.0;
        let mut file_active: File = bookcase.open_book(fid_active, false, true);


        let mut thread_work: WorkIterator = WorkUnitIterator::new(start, stop);

        while let Some(work) = thread_work.next() {   // We have one (file,page) and
            let fid = work.0;                         // create the associated page in the
            let pid = work.1;                         // write buffer.
            
            if fid_active != fid {
                // Complete any outstanding writes for this file
                do_write::<WORDS>(&page_buffer, &mut file_active);

                // Open new file for writing.
                fid_active = fid;
                file_active = bookcase.open_book(fid_active, false, true);
            }

<<<<<<< Updated upstream
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
     /* 
    for bid in 0..bookcase.book_count() {
        let file: File = bookcase.open_book(bid, false, true);
        let file_descriptor = file.as_raw_fd();
        for pid in 0..(bookcase.page_count() as usize) {
            let page: Page<DATA_WORDS> = Page::new(preseed, bid as u32, pid as u64);
            let mut source_buffer: [u8; PAGE_SIZE] = to_byte_slice(&page);
            let file_offset: isize = (PAGE_SIZE * pid) as isize;
            let request_tag: u64 = ((pid << 16) | bid as usize) as u64;
            let request_code: IoCmd = IoCmd::Pwrite;

            let request = AioRequest::new()
                .add_fd(file_descriptor)
                .add_offset(file_offset)
                .add_tag(request_tag)
                .add_opcode(request_code)
                .add_buffer(&mut source_buffer);

            let mut requests: [AioRequest; 1] = [request];
            let ret = aio_submit(ctx, &mut requests);
            if ret.is_err() { panic!("Failed to submit request for page {} in book {}!", pid, bid); }
        }
    }
    */
=======
            let page: &mut Page<WORDS> = &mut page_buffer[wb_idx];
            page.reinit(0xdead, fid, pid);
            wb_idx += 1;

            if wb_idx == page_buffer.len() {
                do_write::<WORDS>(&page_buffer, &mut file_active);
            }
        }
    }
}
>>>>>>> Stashed changes

fn data_verify(queue: WorkerQueue, bookcase: Arc<BookCase>) {
    while let Some(range) = queue.next() {
        let mut page_buffer: Box<[Page<WORDS>; PAGES_PER_WRITE]> = Box::new([Page::default(); PAGES_PER_WRITE]);
        let mut read_buffer: Box<[u8; PAGES_PER_WRITE * PAGE_SIZE]> = Box::new([0; PAGES_PER_WRITE * PAGE_SIZE]); 
        let mut wb_idx: usize = 0;

        let start: WorkUnit = range.0;
        let stop: WorkUnit = range.1;

        let mut fid_active: u32 = start.0.0;
        let mut file_active: File = bookcase.open_book(fid_active, false, true);


        let mut thread_work: WorkIterator = WorkUnitIterator::new(start, stop);

        while let Some(work) = thread_work.next() {   // We have one (file,page) and
            let fid = work.0;                         // create the associated page in the
            let pid = work.1;                         // write buffer.
            
            if fid_active != fid {
                // Complete any outstanding writes for this file
                let page_buffer: &[Page<WORDS>; PAGES_PER_WRITE] = do_read::<WORDS>(&mut read_buffer, &mut file_active);
                for page in page_buffer.iter() {
                    assert!(page.validate_page_with(0xdead, fid, pid));
                }

                // Open new file for writing.
                fid_active = fid;
                file_active = bookcase.open_book(fid_active, true, false);
            }

            let page: &mut Page<WORDS> = &mut page_buffer[wb_idx];
            page.reinit(0xdead, fid, pid);
            wb_idx += 1;

            if wb_idx == page_buffer.len() {
                let page_buffer: &[Page<WORDS>; PAGES_PER_WRITE] = do_read::<WORDS>(&mut read_buffer, &mut file_active);
                for page in page_buffer.iter() {
                    assert!(page.validate_page_with(0xdead, fid, pid));
                }
            }
        }
    }
}

pub fn create_pages_from_queue() {
    use stacker::remaining_stack;
    println!("starting!");

    let pprefix: String = String::from("/home/chuck/programming/testing");
    let dprefix: String = String::from("shelf");
    let fprefix: String = String::from("book");

    let bookcase: Arc<BookCase> = Arc::new(
            BookCase::new(pprefix.to_owned(),
                          dprefix.to_owned(),
                          DIRECTORY_COUNT as u32,
                          fprefix.to_owned(),
                          FILE_COUNT as u32,
                          PAGE_SIZE,
                          PAGE_COUNT as u64)
            );
    bookcase.construct().expect("Could not create test bookcase structures.");

    println!("[Stack] pre-thread_write: {}", remaining_stack().unwrap());
    thread_write(Arc::new(0.into()), bookcase.clone());
    println!("[Stack] pre_data_verify: {}", remaining_stack().unwrap());
    data_verify(Arc::new(0.into()), bookcase.clone());
    println!("[Stack] post-data_verify: {}", remaining_stack().unwrap());

    bookcase.demolish().expect("Could not demolish test bookcase");
    println!("finished!");
    assert!(true);
}

fn main() -> Result<()> {
    use stacker::remaining_stack;
    println!("[Stack] Start: {}", remaining_stack().unwrap());
    create_pages_from_queue();
    println!("[Stack] End: {}", remaining_stack().unwrap());

    Ok(())
}
