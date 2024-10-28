use std::{ 
    fs::File, io::{ 
        Read, 
        Seek, 
        SeekFrom, 
        Write 
    }, 
    path::PathBuf, 
    sync::Arc, 
    time::{ Duration, SystemTime }
};
use clap::{
    parser::ValueSource, 
    value_parser, 
    Arg, 
    ArgAction,
//    ArgGroup,
    ArgMatches,
    Command,
    ValueHint
};
use anyhow::Result;
use log::{info,debug,warn};

//use perfcnt::{AbstractPerfCounter, PerfCounter};
//use perfcnt::linux::{PerfCounterBuilderLinux, HardwareEventType};

use indicatif::{HumanBytes, HumanDuration};
use rayon::{
    iter::{
        IntoParallelIterator,
        ParallelIterator as _
    }, 
    ThreadPool
};
use super_massive_io::{
    bookcase::{BookCasePlans, BookCase},
    chapter::Chapter,
    page::Page,
    queue::{
        AccessPattern, 
        Queue, 
        SerialAccess, 
        WorkQueue
    }, 
    Inspector, 
    PAGES_PER_CHAPTER, 
    PAGE_BYTES
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
                .long("teardown")
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
                .default_value("./")
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
        
        // TODO: Temporary
        .arg(
            Arg::new("verify")
                .long("verify")
                .action(ArgAction::SetTrue)
                .help("Use slow, single-threaded, but sound read/verify function to check writes.")
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
fn setup_bookcase(matches: ArgMatches) -> Result<BookCase> {

    let bookcase: BookCase;
    if let Some(config_file) = matches.get_one::<PathBuf>("config") {
        let cfile: &str = config_file.to_str().unwrap();

        bookcase = BookCasePlans::from_configuration_file(cfile)?.construct()?;
    } 
    else {
        if matches.value_source("path-prefix")
                  .is_some_and(|source| source != ValueSource::CommandLine) {
            warn!("Using default value for prefix path");
        }
        

        // Set up the file structure
        let pprefix: &PathBuf = matches.get_one::<PathBuf>("path-prefix").unwrap();
        let dprefix: String  = matches.get_one::<String>("directory-prefix").unwrap().to_string();
        let fprefix: String  = matches.get_one::<String>("book-prefix").unwrap().to_string();
        let dcount: u64      = *matches.get_one("directory-count").unwrap();
        let fcount: u64      = *matches.get_one("book-count").unwrap();
        let pcount: u64      = *matches.get_one("page-count").unwrap();
        //let direct_io: bool = *matches.get_one("o_direct").unwrap();

        let bookcase_plan: BookCasePlans = BookCasePlans::new(pprefix.to_owned(),
                                                              dprefix.to_owned(),
                                                              dcount,
                                                              fprefix.to_owned(),
                                                              fcount, PAGE_BYTES * pcount as usize).expect("check path arguments");

        //if direct_io { bookcase.use_direct_io() };

        bookcase_plan.to_configuration_file()?;
        bookcase = bookcase_plan.construct()?;
    }
    
    Ok(bookcase)
}

fn setup_threads() -> (ThreadPool, usize) {
    // Set up thread pool
    let available_cpus: usize = std::thread::available_parallelism().unwrap().into();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(available_cpus)
                                              .build()
                                              .unwrap();
    (pool, available_cpus)
}

pub enum Mode {
    Create,
    Bench,
    Verify,
    Teardown,
} impl Mode {
    fn to_str(&self) -> &str {
        match self {
            Mode::Create => "Create",
            Mode::Bench => "Bench",
            Mode::Verify => "Verify",
            Mode::Teardown => "Teardown",
        }

    }
}

fn main() -> Result<()> {

    // Set up logging
    fern::Dispatch::new()
        .format(|out,message,record| {
            out.finish(format_args!(
                    "[{}][{}][{}]{}",
                    humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                    record.level(),
                    record.target(),
                    message
            ))
                    
        })
    .level(log::LevelFilter::Info)
    .chain(std::io::stdout())
    .apply()?;

    let args: ArgMatches = cli_arguments().get_matches();
    let mut modes: Vec<Mode> = Vec::new();
    if *args.get_one("create").unwrap()    { modes.push(Mode::Create); }
    if *args.get_one("bench").unwrap()     { modes.push(Mode::Bench); }
    if *args.get_one("verify").unwrap()    { modes.push(Mode::Verify); }
    if *args.get_one("tear-down").unwrap() { modes.push(Mode::Teardown); }

    let pcount: u64 = *args.get_one("page-count").expect("[Error] Developer Error: no page count");
    let seed: u64 = *args.get_one("seed").expect("[Error] Seed must be provided as a integer");

    let bookcase: BookCase = setup_bookcase(args).expect("[Error] Could not setup file structure!");

    // This should check if bookcase even needs creating
    let fcount = bookcase.book_count();

    const P: usize = PAGES_PER_CHAPTER;
    const W: usize = PAGE_BYTES / 8 - 4;
    const B: usize = Page::<W>::PAGE_BYTES * P;
    let serial: SerialAccess = SerialAccess::new(0, fcount * pcount, 1, PAGES_PER_CHAPTER as u64);

    let (pool, cpus): (ThreadPool, usize) = setup_threads();


    modes.iter()
        .for_each(|mode| { 
            let mode_start: SystemTime = SystemTime::now();
            match mode {
               Mode::Create | Mode::Bench => {
                   let queue: Queue<SerialAccess> = Queue::new(pcount, serial.clone());
                   let chapter = Box::new(Chapter::<P,W,B>::new());
                   let metrics: Arc<Inspector> = Arc::new(Inspector::new(cpus));
                   pool.install(|| {
                       (0..cpus).into_par_iter()
                                .for_each(|_|{
                                    thread_worker::<P,W,B,SerialAccess>(seed, 
                                                                        mode, 
                                                                        queue.clone(), 
                                                                        chapter.clone(), 
                                                                        bookcase.clone(),
                                                                        metrics.clone()
                                                                        );
                                });
                   });
                   let _ = metrics.flush();
                   
                   info!(target: "performance", "[{}] Total written: {}", mode.to_str(), metrics.get_report_global());
               },
               Mode::Verify   => single_threaded_verify(&bookcase),
               Mode::Teardown => { let _ = bookcase.deconstruct().expect("Could not teardown files setup by application"); },
            }

            let mode_time: u128 = mode_start.elapsed().unwrap().as_nanos();
            info!(target: "performance", "[{}] Total time: {}ns", mode.to_str(), mode_time);
        });
    Ok(())
}


fn thread_worker<const P:usize,const W: usize,const B: usize,T: AccessPattern>(
     seed: u64, 
     mode: &Mode,
     queue: Queue<T>,
     mut chapter: Box<Chapter<P,W,B>>,
     bookcase: BookCase,
     metrics: Arc<Inspector>
) {
    let thread_id: usize = rayon::current_thread_index().unwrap_or(0);
    let fcount = bookcase.book_count();
    let pcount: u64 = (bookcase.book_size() / PAGE_BYTES) as u64;
    let is_read: bool = match mode {
        Mode::Bench => true,
        _ => false
    };

    metrics.register_thread();

    let mode_start: SystemTime = SystemTime::now();
    let mut sample_timer: SystemTime = SystemTime::now();

    while let Some((page_id, book_id)) = queue.take_work() {

        if page_id * book_id >= queue.capacity() { break; }
        if book_id >= fcount                     { break; }
        if page_id >= pcount                     { break; }
        let mut book_file: File = bookcase.open_book(book_id, is_read, !is_read)
                                          .expect("Could  not open  file!");

        if page_id != 0 {
            book_file.seek(SeekFrom::Start(page_id * PAGE_BYTES as u64))
                     .expect(&format!("Unable to seek to write location in book {book_id}"));
        }

        if is_read {
            let buffer: &mut [u8] = chapter.mutable_bytes_all();
            let bytes_read: usize = book_file.read(buffer).expect("Could not read from file!");
            if bytes_read == 0 || bytes_read % PAGE_BYTES != 0 { break; } // This should emit a debug
        }  
        
        let start: u64 = page_id;
        let end: u64   = start + queue.chunk_size() ;

        if end <= pcount {
            (start..end).for_each(|p|{
                if is_read {
                    if !chapter.mutable_page(p % queue.chunk_size()).is_valid() {
                        let (s, f, p, m) = chapter.mutable_page(p % queue.chunk_size()).get_metadata();
                        warn!("Invalid Page Found: book {book_id}, page {page_id}");
                        warn!("Seed: 0x{s:X}\nFile: 0x{f:X}\nPage: 0x{p:X}\nMutations: 0x{m:X}");
                    } 
                } else {
                    chapter.mutable_page(p % queue.chunk_size())
                           .reinit(seed, book_id, p, 0);
                }
            });

            if !is_read {
                book_file.write_all(chapter.bytes_all()).unwrap();
                book_file.flush().expect("Could not flush file");
            }

            let bytes_completed: u64 = chapter.byte_count() as u64;
            if metrics.update(bytes_completed).is_err() {
                warn!(target: "performance", "[{}][tid:{}] Unable to update bytes completed for thread", mode.to_str(), thread_id);

            }
        }

        if thread_id == 0 && 999 <= sample_timer.elapsed().unwrap().as_millis() {
            if metrics.flush().is_ok() { 
                // TODO: This reporting should be integrated into Inspector
                let total_work = metrics.get_global_total();
                let total_time = mode_start.elapsed().unwrap();
                info!(target: "performance", "[{}][tid:{}] {}, {}, {}/s, {:.2} ns/byte", 
                                            mode.to_str(),
                                            thread_id,
                                            HumanBytes(total_work),
                                            HumanDuration(total_time),
                                            HumanBytes((total_work as f64 / total_time.as_secs() as f64) as u64),
                                            total_time.as_nanos() as f64 / total_work as f64);
            } else {
                warn!(target: "performance", "[{}][tid:{}] Unable sync metrics between threads", mode.to_str(), thread_id) 
            }
            sample_timer = SystemTime::now();
        }
    }
}


// Keep this function around as a secondary check on 
// multi-threaded read and verify.
// Eventually this should be replaced with a multi-threaded
// monotonic read-only access pattern worker
fn single_threaded_verify(bookcase: &BookCase) {
    let fcount = bookcase.book_count();

    const P: usize = PAGES_PER_CHAPTER;
    const W: usize = PAGE_BYTES / 8 - 4;
    const B: usize = Page::<W>::PAGE_BYTES * P;
    let mut work: u64 = 0;

    let mut chapter = Box::new(Chapter::<P,W,B>::new());
    let now: SystemTime = SystemTime::now();

    // Read from a File
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
                                    }
                               });
                        work += chapter.byte_count() as u64;
                    }
               });
    let elapsed: Duration = now.elapsed().unwrap();
    let nanos = elapsed.as_nanos();
    println!("[tid:{}][read] {}, {}, {} ns/byte", rayon::current_thread_index().unwrap_or(0),
                                                     HumanDuration(elapsed),
                                                     HumanBytes(work),
                                                     nanos / work as u128);
}
