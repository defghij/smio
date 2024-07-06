use std::{
    io::{
        Result,
        Write,
        Read,
    },
    fs::File
};
use clap::{
    ArgMatches,
    value_parser,
    Arg,
//    ArgGroup,
    Command
};

use scribe::{
    bookcase::BookCase,
    PAGE_SIZE,
    PAGES_PER_CHAPTER,
    page::Page,
    chapter::Chapter
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
                                               PAGE_SIZE,
                                               pcount);

    bookcase.construct().expect("Could not construct bookcase");



    const P: usize = PAGES_PER_CHAPTER;
    const W: usize = PAGE_SIZE / 8 - 4;
    const B: usize = Page::<W>::PAGE_BYTES * P;

    let mut chapter = Box::new(Chapter::<P,W,B>::new());

    // Write to a File
    (0..fcount).into_iter()
               .for_each(|book| { 
                    let mut writable_book: File = bookcase.open_book(book, false, true).expect("Could  not open  file!");
                   
                    let full_writes: u64 = pcount / PAGES_PER_CHAPTER as u64;
                    let partial_writes: u64 = pcount % PAGES_PER_CHAPTER as u64;

                    println!("Book {book} writes: full {full_writes}, partial {partial_writes}");

                    (0..full_writes).into_iter()
                                    .for_each(|fwrite|{
                                        let start: u64 = fwrite * PAGES_PER_CHAPTER as u64;
                                        let end: u64 = start + PAGES_PER_CHAPTER as u64 ;

                                        println!("Full Write: ({start},{end}) @ file{book}");

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

                        println!("Partial Write: ({partial_start},{partial_end}) @ file{book}");

                        (partial_start..partial_end).for_each(|p|{
                            chapter.mutable_page(p % partial_writes)
                                   .reinit(seed, book, p, 0);
                        });

                        let pages_to_write: usize = (partial_end - partial_start) as usize;
                        let partial_byte_count: usize = PAGE_SIZE * pages_to_write;


                        writable_book.write_all(chapter.bytes_upto(partial_byte_count)).unwrap();
                        writable_book.flush().expect("Could not flush file");

                        chapter.zeroize();
                    }
                    drop(writable_book);
               });

    // Read from a File
    (0..fcount).into_iter()
               .for_each(|book| { 
                    let mut readable_book: File = bookcase.open_book(book, true, false).expect("Could  not open  file!");

                    loop {
                        let writable_buffer: &mut [u8] = chapter.mutable_bytes_all();
                        let bytes_read: usize = readable_book.read(writable_buffer).expect("Could not read from file!");

                        if bytes_read == 0 || bytes_read % PAGE_SIZE != 0 { break; }

                        chapter.pages_all()
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
    Ok(())
}
