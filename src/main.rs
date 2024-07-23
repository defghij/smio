use std::{ 
    fs::File, io::{ Read, Result, Seek, SeekFrom, Write }, path::PathBuf, sync::{atomic::AtomicU64, Arc}, thread, time::SystemTime
};
use clap::{
    parser::ValueSource, value_parser, Arg, ArgAction, ArgMatches, Command, ValueHint
};

use rayon::iter::{IntoParallelIterator, ParallelIterator as _};
use serde_json::Value;
use SuperMassiveIO::{
    bookcase::BookCase,
    PAGE_BYTES,
    PAGES_PER_CHAPTER,
    page::Page,
    chapter::Chapter,
    WorkQueue
};
 

fn cli_arguments() -> Command {

    Command::new("SuperMassiveIO")
        .about("Research application into File System IO")
        .version("0.1.0")
        .author("defghij")
        // Common Arguments
        .arg(
            Arg::new("verbosity")
                .short('v')
                .long("verbosity")
                .value_parser(["none", "info", "debug", "warning"])
                .default_value("none")
                .default_missing_value("info")
                .help("Verbosity level of the application")
         )
        .arg(
            Arg::new("seed")
                .short('s')
                .long("seed")
                .default_value("15552853473234178512") /*0xD7D6D5D4D3D2D1D0*/
                .value_parser(value_parser!(u64))
                .value_name("integer")
                .value_hint(ValueHint::Other)
                .help("Seed value used to generate page data.")
         )

        // FILE LAYOUT
        // Size, layout, count
        .arg(
            Arg::new("page-size")
                .short('P')
                .long("page-size")
                .default_value("4096")
                .value_parser(value_parser!(usize))
                .value_name("integer")
                .value_hint(ValueHint::Other)
                .help("The number of bytes a page must contain.")
        )
        .arg(
            Arg::new("page-count")
                .short('p')
                .long("page-count")
                .default_value("512")
                .value_parser(value_parser!(u64))
                .value_name("integer")
                .value_hint(ValueHint::Other)
                .help("Size of a page as specified by $2^{exponent}$ bytes.")
        )
        // TODO: This should conflict with -p and -P
        .arg(
            Arg::new("book-size")
                .short('F')
                .long("file-size")
                .default_value("2097152")
                .value_parser(value_parser!(usize))
                .value_name("integer")
                .value_hint(ValueHint::Other)
                .help("Size of files as specified by $2^{exponent}$ bytes. If not a multiple of the page size, the remaining bytes will be be dropped")
        )
        .arg(
            Arg::new("book-count")
                .short('f')
                .long("file-count")
                .default_value("1")
                .value_parser(value_parser!(u64))
                .value_name("integer")
                .value_hint(ValueHint::Other)
                .help("Number of books (files) to create.")
        )
        .arg(
            Arg::new("book-prefix")
                .long("file-prefix")
                .default_value("book")
                .value_parser(value_parser!(String))
                .value_name("string")
                .value_hint(ValueHint::Other)
                .help("Prefix for generated books (files). Will have form 'prefix##'")
        )

        // DIRECTORY LAYOUT
        //// Specify prefix and count
        .arg(
            Arg::new("directory-count")
                .short('d')
                .long("directory-count")
                .default_value("1")
                .value_parser(value_parser!(u64))
                .value_name("integer")
                .value_hint(ValueHint::Other)
                .help("Number of generated directories.")
        )
        .arg(
            Arg::new("directory-prefix")
                .long("directory-prefix")
                .default_value("shelf")
                .value_parser(value_parser!(String))
                .value_name("string")
                .value_hint(ValueHint::Other)
                .help("Prefix for generated directories. Will have the form 'prefix##'")
        )
        .arg(
            Arg::new("path-prefix")
                .long("path-prefix")
                .default_value("/tmp")
                .value_parser(value_parser!(PathBuf))
                .value_name("path")
                .value_hint(ValueHint::FilePath)
                .help("Path to the root (parent) directory for the books (directories) of the program input/output.")
        )

        // Write Characterization
        .arg(
            Arg::new("o_direct")
                .long("direct-io")
                .action(ArgAction::SetTrue)
                .help("Will use direct IO (O_DIRECT). Will error if not available or incompatable with other options.")
        )
        // Config File Path: TODO
        //.arg(
        //    Arg::new("config")
        //        .long("configuration-file")
        //        .value_parser(value_parser!(PathBuf))
        //        .value_name("path")
        //        .value_hint(ValueHint::FilePath)
        //        .help("Path to file that can be used in place of CLI arguments. Note: CLI arguments have precedence.")
        //)

} 


fn main() -> Result<()> {

    let matches: ArgMatches = cli_arguments().get_matches();

    if let Some(c) = matches.get_one::<String>("verbosity") {
        println!("[Info] Verbosity Level: {c}");
    }

    if matches.value_source("path-prefix")
              .is_some_and(|source| source != ValueSource::CommandLine)
    {
        println!("[Warn] Using default value for prefix path");

    }


    // Set up the file structure
    let pprefix: &PathBuf = matches.get_one::<PathBuf>("path-prefix").unwrap();
    let dprefix: String = matches.get_one::<String>("directory-prefix").unwrap().to_string();
    let fprefix: String = matches.get_one::<String>("book-prefix").unwrap().to_string();
    let dcount: u64     = *matches.get_one("directory-count").unwrap();
    let fcount: u64     = *matches.get_one("book-count").unwrap();
    let pcount: u64     = *matches.get_one("page-count").unwrap();
    let seed: u64       = *matches.get_one("seed").unwrap();
    let direct_io: bool = *matches.get_one("o_direct").unwrap();


    let mut bookcase: BookCase = BookCase::new(pprefix.to_owned(), 
                                               dprefix.to_owned(),
                                               dcount,
                                               fprefix.to_owned(),
                                               fcount,
                                               PAGE_BYTES,
                                               pcount,
                                               seed);
    if direct_io { bookcase.use_direct_io() };
    bookcase.construct()?;
    let fcount = bookcase.book_count();
    let pcount = bookcase.page_count();

    const P: usize = PAGES_PER_CHAPTER;
    const W: usize = PAGE_BYTES / 8 - 4;
    const B: usize = Page::<W>::PAGE_BYTES * P;
    let queue: WorkQueue = WorkQueue::new(fcount*pcount, PAGES_PER_CHAPTER as u64, pcount);
    let chapter = Box::new(Chapter::<P,W,B>::new());


    // Set up thread pool
    let available_cpus: usize = std::thread::available_parallelism().unwrap().into();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(available_cpus)
                                              .build()
                                              .unwrap();

    pool.install(|| {
        (0..available_cpus).into_par_iter()
                           .for_each(|_|{
                               do_write_work::<P,W,B>(queue.clone(), chapter.clone(), bookcase.clone());
                           });
    });

    //multi_threaded_write(&mut bookcase, seed);
    multi_threaded_read(&mut bookcase, seed + 1);
    bookcase.demolish()?;

    Ok(())
}

fn do_write_work<const P:usize, const W: usize, const B: usize>(queue: WorkQueue, mut chapter: Box<Chapter<P,W,B>>, bookcase: BookCase) {
    let seed: u64 = bookcase.seed;
    let fcount = bookcase.book_count();
    let pcount = bookcase.page_count();
    let mut work: u64 = 0;

    let now: SystemTime = SystemTime::now();

    while let Some((page, book)) = queue.take_work() {

        if page * book >= queue.capacity { break; }
        if book >= fcount                { break; }
        if page >= pcount                { break; }

        let mut writable_book: File = bookcase.open_book(book, false, true).expect("Could  not open  file!");
        if page != 0 {
            writable_book.seek(SeekFrom::Start(page * PAGE_BYTES as u64))
                         .expect("Unable to seek to write location in book");
        }
        
        let start: u64 = page;
        let end: u64   = start + queue.step ;

        if end <= pcount {
            (start..end).for_each(|p|{
                chapter.mutable_page(p % queue.step)
                              .reinit(seed, book, p, 0);
            });

            writable_book.write_all(chapter.bytes_all()).unwrap();
            writable_book.flush().expect("Could not flush file");
            work += chapter.byte_count() as u64;
        }
    }

    let duration: u128 = now.elapsed().unwrap().as_millis();
    println!("[tid:{}] Spent {}ms writing {} bytes", rayon::current_thread_index().unwrap(),
                                                     duration,
                                                     work);

}

fn multi_threaded_write(bookcase: &BookCase, seed: u64) {
    let fcount = bookcase.book_count();
    let pcount = bookcase.page_count();

    const P: usize = PAGES_PER_CHAPTER;
    const W: usize = PAGE_BYTES / 8 - 4;
    const B: usize = Page::<W>::PAGE_BYTES * P;

    let chapter = Box::new(Chapter::<P,W,B>::new());

    let queue: WorkQueue = WorkQueue::new(fcount*pcount, PAGES_PER_CHAPTER as u64, pcount);
    let mut handles: Vec<thread::JoinHandle<_>> = Vec::new();

    let now: SystemTime = SystemTime::now();

    (0..8).for_each(|_|{
        let thread_queue = queue.clone();
        let thread_bookcase = bookcase.clone();
        let mut thread_chapter = chapter.clone();
        
        let handle = thread::spawn(move || {

            while let Some((page, book)) = thread_queue.take_work() {

                if page * book >= thread_queue.capacity { break; }
                if book >= fcount                       { break; }
                if page >= pcount                       { break; }

                let mut writable_book: File = thread_bookcase.open_book(book, false, true).expect("Could  not open  file!");
                if page != 0 {
                    writable_book.seek(SeekFrom::Start(page * PAGE_BYTES as u64))
                                 .expect("Unable to seek to write location in book");
                }
                
                let start: u64 = page;
                let end: u64   = start + thread_queue.step ;

                if end <= pcount {
                    (start..end).for_each(|p|{
                        thread_chapter.mutable_page(p % thread_queue.step)
                                      .reinit(seed, book, p, 0);
                    });

                    writable_book.write_all(thread_chapter.bytes_all()).unwrap();
                    writable_book.flush().expect("Could not flush file");
                }
            }
        });
        handles.push(handle);
    });

    for handle in handles {
        handle.join().expect("Cant join");

    }

    let duration: u128 = now.elapsed().unwrap().as_millis();
    println!("Spent {}ms writing {} bytes", duration, fcount * pcount * PAGE_BYTES as u64);
}

fn multi_threaded_read(bookcase: &BookCase, _seed: u64) {
    let fcount = bookcase.book_count();
    let pcount = bookcase.page_count();

    const P: usize = PAGES_PER_CHAPTER;
    const W: usize = PAGE_BYTES/ 8 - 4;
    const B: usize = Page::<W>::PAGE_BYTES * P;

    let chapter = Box::new(Chapter::<P,W,B>::new());

    let queue: WorkQueue = WorkQueue::new(fcount*pcount, PAGES_PER_CHAPTER as u64, pcount);
    let mut handles: Vec<thread::JoinHandle<_>> = Vec::new();

    let now: SystemTime = SystemTime::now();

    (0..8).for_each(|_|{
        let thread_queue = queue.clone();
        let thread_bookcase = bookcase.clone();
        let mut thread_chapter = chapter.clone();
        
        let handle = thread::spawn(move || {

            while let Some((page, book)) = thread_queue.take_work() {

                if page * book >= thread_queue.capacity { break; }
                if book >= fcount                       { break; }
                if page >= pcount                       { break; }

                let mut readable_book: File = thread_bookcase.open_book(book, true, false).expect("Could  not open  file!");

                let writable_buffer: &mut [u8] = thread_chapter.mutable_bytes_all();
                let bytes_read: usize = readable_book.read(writable_buffer).expect("Could not read from file!");

                if bytes_read == 0 || bytes_read % PAGE_BYTES != 0 { break; }

                thread_chapter.pages_all()
                       .iter()
                       .for_each(|page|{
                            if !page.is_valid() {
                                let (s, f, p, m) = page.get_metadata();
                                println!("Invalid Page Found: book {book}, page {page}");
                                println!("Seed: 0x{s:X}\nFile: 0x{f:X}\nPage: 0x{p:X}\nMutations: 0x{m:X}");
                            }
                       });
            }
        });
        handles.push(handle);
    });

    for handle in handles {
        handle.join().expect("Cant join");

    }

    let duration: u128 = now.elapsed().unwrap().as_millis();
    println!("Spent {}ms reading {} bytes", duration, fcount * pcount * PAGE_BYTES as u64);
}
