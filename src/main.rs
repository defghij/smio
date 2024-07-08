use std::{ 
    fs::File, io::{ Read, Result, Seek, SeekFrom, Write }, sync::{atomic::AtomicU64, Arc}, thread, time::SystemTime
};
use clap::{
    ArgMatches,
    value_parser,
    Arg,
//    ArgGroup,
    Command
};

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
                .help("The number of bytes a page must contain.")
        )
        .arg(
            Arg::new("page-count")
                .short('p')
                .long("page-count")
                .default_value("512")
                .value_parser(value_parser!(u64))
                .help("The number of bytes a page must contain.")
        )
        .arg(
            Arg::new("book-size")
                .short('F')
                .long("file-size")
                .default_value("2097152")
                .value_parser(value_parser!(usize))
                .help("Size of files in bytes. If not a multiple of the page size, the remaining bytes will be be dropped")
        )
        .arg(
            Arg::new("book-count")
                .short('f')
                .long("file-count")
                .default_value("1")
                .value_parser(value_parser!(u64))
                .help("Number of books (files) to create.")
        )
        .arg(
            Arg::new("book-prefix")
                .long("file-prefix")
                .default_value("book")
                .value_parser(value_parser!(String))
                .help("String prefix for generated books (files).")
        )

        // DIRECTORY LAYOUT
        //// Specify prefix and count
        .arg(
            Arg::new("directory-count")
                .short('d')
                .long("directory-count")
                .default_value("1")
                .value_parser(value_parser!(u64))
                .help("Number of generated directories.")
        )
        .arg(
            Arg::new("directory-prefix")
                .long("directory-prefix")
                .default_value("shelf")
                .value_parser(value_parser!(String))
                .help("String prefix for generated directories.")
        )
        //// Manually specify list of directories
        .arg(
            Arg::new("directory-list")
                .long("directory-list")
                .value_parser(value_parser!(String))
                .help("A comma separated list of directories to be created and used for book generation. Cannot be used with `--directory-prefix` & `--directory-count`.")
        )
} 


fn main() -> Result<()> {

    let matches: ArgMatches = cli_arguments().get_matches();

    if let Some(c) = matches.get_one::<String>("verbosity") {
        println!("Verbosity Level: {c}");
    }

    // For testing. Unwrap is safe due to default values.
    let pprefix: String = String::from("../testing/case");
    let dprefix: String = matches.get_one::<String>("directory-prefix").unwrap().to_string();
    let fprefix: String = matches.get_one::<String>("book-prefix").unwrap().to_string();
    let dcount: u64     = *matches.get_one("directory-count").unwrap();
    let fcount: u64     = *matches.get_one("book-count").unwrap();
    let pcount: u64     = *matches.get_one("page-count").unwrap();
    let seed: u64       = *matches.get_one("seed").unwrap();
    let mut bookcase: BookCase = BookCase::new(pprefix.to_owned(), 
                                               dprefix.to_owned(),
                                               dcount,
                                               fprefix.to_owned(),
                                               fcount,
                                               PAGE_BYTES,
                                               pcount);



    bookcase.construct()?;
    multi_threaded_write(&mut bookcase, seed);
    multi_threaded_read(&mut bookcase, seed);
    bookcase.demolish()?;

    Ok(())
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
