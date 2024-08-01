use std::{ 
    fs::File, io::{ Read, Result, Seek, SeekFrom, Write }, path::PathBuf, sync::{atomic::AtomicU64, Arc}, thread, time::SystemTime
};
use clap::{
    parser::ValueSource, value_parser, Arg, ArgAction, ArgGroup, ArgMatches, Command, ValueHint
};

use rayon::{iter::{IntoParallelIterator, ParallelIterator as _}, ThreadPool};
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
            Arg::new("create")
                .long("create")
                .action(ArgAction::SetTrue)
                .conflicts_with("config")
                .help("Enables the creation of data.")
        )
        .arg(
            Arg::new("bench")
                .long("bench")
                .action(ArgAction::SetTrue)
                .help("Enables the benchmarking mode. Requires '--configuration-file' if not used with 'create' flag")
        )
        .arg(
            Arg::new("verbosity")
                .short('v')
                .long("verbosity")
                .value_parser(["none", "info", "debug", "warning"])
                .default_value("none")
                .default_missing_value("info")
                .groups(["creation", "benchmarking"])
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
                .group("creation")
                .help("Seed value used to generate page data.")
         )
        .arg(
            Arg::new("config")
                .long("configuration-file")
                .value_parser(value_parser!(PathBuf))
                .value_name("path")
                .value_hint(ValueHint::FilePath)
                .group("benchmarking")
                .help("Path to file that can be used in place of CLI arguments. Note: CLI arguments have precedence.")
        )
        .arg(
            Arg::new("tear-down")
                .long("destroy-after")
                .action(ArgAction::SetTrue)
                .groups(["creation", "benchmarking"])
                .help("Will cause the deletion of test data after completion of the process.")
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
                .conflicts_with("config")
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
                .conflicts_with("config")
                .help("Size of a page as specified by $2^{exponent}$ bytes.")
        )
        .arg(
            Arg::new("book-size")
                .short('F')
                .long("file-size")
                .default_value("2097152")
                .value_parser(value_parser!(usize))
                .value_name("integer")
                .value_hint(ValueHint::Other)
                .conflicts_with_all(["page-size", "page-count", "config"])
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
                .conflicts_with("config")
                .help("Number of books (files) to create.")
        )
        .arg(
            Arg::new("book-prefix")
                .long("file-prefix")
                .default_value("book")
                .value_parser(value_parser!(String))
                .value_name("string")
                .value_hint(ValueHint::Other)
                .conflicts_with("config")
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
                .conflicts_with("config")
                .help("Number of generated directories.")
        )
        .arg(
            Arg::new("directory-prefix")
                .long("directory-prefix")
                .default_value("shelf")
                .value_parser(value_parser!(String))
                .value_name("string")
                .value_hint(ValueHint::Other)
                .conflicts_with("config")
                .help("Prefix for generated directories. Will have the form 'prefix##'")
        )
        .arg(
            Arg::new("path-prefix")
                .long("path-prefix")
                .default_value("/tmp/smio")
                .value_parser(value_parser!(PathBuf))
                .value_name("path")
                .value_hint(ValueHint::FilePath)
                .conflicts_with("config")
                .help("Path to the root (parent) directory for the books (directories) of the program input/output.")
        )

        // Write Characterization
        .arg(
            Arg::new("engine")
                .long("engine")
                .value_parser(["posix", "direct_io", "mmap", "libaio", "io_uring"])
                .help("Select the file IO interface to use.")
        )
} 

/// This function handles all aspects of creating the application context
/// type BookCase. This can be either from a configuration file or from
/// commandline arguments.
/// TODO: 
/// - Overwrite config parameters if cli ones are provided
/// - This should probably return a Result<T,E>
///     Possible Errors: 
///         - Cannot Create BookCase
///         - Config File isn't valid
///         - Config File doesn't point valid file structures.
fn setup_bookcase(matches: ArgMatches) -> BookCase {
    if let Some(c) = matches.get_one::<String>("verbosity") {
        println!("[Info] Verbosity Level: {c}");
    }

    let mut bookcase: BookCase;
    if let Some(config_file) = matches.get_one::<PathBuf>("config") {
        bookcase = BookCase::from_string(config_file.to_str().unwrap());
        if !bookcase.is_assembled() {
            println!("Configuration expected directory/file structure which cannot be found. Creating new one.");
            bookcase.construct().unwrap();
        }
    } 
    else {
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
        //let direct_io: bool = *matches.get_one("o_direct").unwrap();


        bookcase = BookCase::new(pprefix.to_owned(), 
                                 dprefix.to_owned(),
                                 dcount,
                                 fprefix.to_owned(),
                                 fcount,
                                 PAGE_BYTES,
                                 pcount,
                                 seed);
        //if direct_io { bookcase.use_direct_io() };

        bookcase.write_configuration_file();
        bookcase.construct().unwrap();
    }

    bookcase
}

fn setup_threads() -> (ThreadPool, usize) {
    // Set up thread pool
    let available_cpus: usize = std::thread::available_parallelism().unwrap().into();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(available_cpus)
                                              .build()
                                              .unwrap();
    (pool, available_cpus)
}


fn main() -> Result<()> {

    let args: ArgMatches = cli_arguments().get_matches();
    let create_mode: bool = *args.get_one("create").unwrap();
    let bench_mode:  bool = *args.get_one("bench").unwrap(); 
    let teardown:    bool = *args.get_one("tear-down").unwrap();
    
    let mut bookcase: BookCase = setup_bookcase(args);

    // This should check if bookcase even needs creating
    let fcount = bookcase.book_count();
    let pcount = bookcase.page_count();

    const P: usize = PAGES_PER_CHAPTER;
    const W: usize = PAGE_BYTES / 8 - 4;
    const B: usize = Page::<W>::PAGE_BYTES * P;
    let queue: WorkQueue = WorkQueue::new(fcount*pcount, PAGES_PER_CHAPTER as u64, pcount);
    let chapter = Box::new(Chapter::<P,W,B>::new());

    let (pool, cpus): (ThreadPool, usize) = setup_threads();

    if create_mode {
        pool.install(|| {
            (0..cpus).into_par_iter()
                               .for_each(|_|{
                                   do_work::<P,W,B>(false, queue.clone(), chapter.clone(), bookcase.clone());
                               });
        });
    }


    // Reset/zero heap state
    queue.clone().reset();
    chapter.clone().zeroize();
     

    if bench_mode {
        pool.install(|| {
            (0..cpus).into_par_iter()
                               .for_each(|_|{
                                   do_work::<P,W,B>(true, queue.clone(), chapter.clone(), bookcase.clone());
                               });
        });

    }

    if teardown {
        bookcase.demolish()?;
    }

    Ok(())
}

fn do_work<const P:usize, const W: usize, const B: usize>(is_read:bool, queue: WorkQueue, mut chapter: Box<Chapter<P,W,B>>, bookcase: BookCase) {
    let seed: u64 = bookcase.seed;
    let fcount = bookcase.book_count();
    let pcount = bookcase.page_count();
    let mut work: u64 = 0;

    let now: SystemTime = SystemTime::now();

    while let Some((page_id, book_id)) = queue.take_work() {

        if page_id * book_id >= queue.capacity { break; }
        if book_id >= fcount                { break; }
        if page_id >= pcount                { break; }

        let mut book_file: File;

        book_file = bookcase.open_book(book_id, is_read, !is_read).expect("Could  not open  file!");

        if page_id != 0 {
            book_file.seek(SeekFrom::Start(page_id * PAGE_BYTES as u64))
                     .expect(&format!("Unable to seek to write location in book {book_id}"));
        }

        if is_read {
            let buffer: &mut [u8] = chapter.mutable_bytes_all();
            let bytes_read: usize = book_file.read(buffer).expect("Could not read from file!");
            if bytes_read == 0 || bytes_read % PAGE_BYTES != 0 { break; } // This should emit a debug
        }
                                                                      // message
        
        let start: u64 = page_id;
        let end: u64   = start + queue.step ;

        if end <= pcount {
            (start..end).for_each(|p|{
                if is_read {
                    if !chapter.mutable_page(p % queue.step).is_valid() {
                        let (s, f, p, m) = chapter.mutable_page(p % queue.step).get_metadata();
                        println!("Invalid Page Found: book {book_id}, page {page_id}");
                        println!("Seed: 0x{s:X}\nFile: 0x{f:X}\nPage: 0x{p:X}\nMutations: 0x{m:X}");
                    } 
                } else {
                    chapter.mutable_page(p % queue.step)
                                  .reinit(seed, book_id, p, 0);
                }
            });
            if !is_read {
                book_file.write_all(chapter.bytes_all()).unwrap();
                book_file.flush().expect("Could not flush file");
            }
            work += chapter.byte_count() as u64;
        }
    }

    let duration: u128 = now.elapsed().unwrap().as_millis();
    if is_read {
        println!("[tid:{}] Spent {}ms reading {} bytes", rayon::current_thread_index().unwrap_or(0),
                                                         duration,
                                                         work);
    } else {
        println!("[tid:{}] Spent {}ms writing {} bytes", rayon::current_thread_index().unwrap_or(0),
                                                         duration,
                                                         work);
    }

}
