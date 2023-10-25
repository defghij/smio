use std::io::Result;
use clap::{
    ArgMatches,
    value_parser,
    Arg,
    ArgGroup,
    Command
};

fn cli_arguments() -> Command {

    Command::new("SuperMassiveIO")
        .about("Research application into File System IO")
        .version("0.1.0")
        .author("defghij")
        .subcommand_required(true)
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

        // FILE LAYOUT
        // Size, layout, count
        .arg(
            Arg::new("page_size")
                .short('p')
                .long("page-size")
                .default_value("4096")
                .value_parser(value_parser!(usize))
                .help("The number of bytes a page must contain.")
        )
        .arg(
            Arg::new("book_size")
                .long("file-size")
                .default_value("2097152")
                .help("Size of files in bytes. If not a multiple of the page size, the remaining bytes will be be dropped")
        )
        .arg(
            Arg::new("book-count")
                .long("file-count")
                .default_value("1")
                .value_parser(value_parser!(usize))
                .help("Number of books (files) to create.")
        )
        .arg(
            Arg::new("book-prefix")
                .long("file-prefix")
                .default_value("file")
                .value_parser(value_parser!(String))
                .help("String prefix for generated books (files).")
        )

        // DIRECTORY LAYOUT
        //// Specify prefix and count
        .arg(
            Arg::new("directory-count")
                .long("directory-count")
                .default_value("1")
                .value_parser(value_parser!(usize))
                .help("Number of generated directories.")
        )
        .arg(
            Arg::new("directory-prefix")
                .long("directory-prefix")
                .default_value("dir")
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
        //// Manual list conflicts with prefix and count
        .group(
            ArgGroup::new("directory-spec")
                .args(["directory-prefix", "directory-count"])
                .multiple(true)
                .conflicts_with("directory-list")
        )
        // Mode: Create
        // Arguments related the to creation of the test data.
        .subcommand(
            Command::new("create")
                .short_flag('C')
                .long_flag("create")
                .about("Created the books (files) used by the application.")
                .arg(
                    Arg::new("output-config")
                        .long("output-configuration")
                        .default_value("false")
                        .value_parser(value_parser!(bool))
                        .help("Write configuration to disk.")
                )
        )
        // Mode: Bench
        // Arguments related to benchmarking
        .subcommand(
            Command::new("bench")
                .short_flag('B')
                .long_flag("Bench")
                .about("Uses books to read/write to/from disk in a deterministic and asynchronous way.")
                .arg(
                    Arg::new("page_size")
                        .short('p')
                        .long("page-size")
                        .default_value("4096")
                        .value_parser(value_parser!(usize))
                        .help("The number of bytes a page must contain.")
                )
                .arg(
                    Arg::new("book_size")
                        .short('f')
                        .long("file-size")
                        .default_value("2097152")
                        .value_parser(value_parser!(usize))
                        .help("Size of files in bytes. If not a multiple of the page size, the remaining bytes will be be dropped")
                )
        )

} 



fn main() -> Result<()> {

    let matches: ArgMatches = cli_arguments().get_matches();
    println!("Here");

    if let Some(c) = matches.get_one::<String>("verbosity") {
        println!("Verbosity Level: {c}");
    }

    match matches.subcommand() {
        Some(("create", create_matches)) => {
            let book_size: usize = *create_matches.get_one("book_size").expect("'file-size is required");
            let page_size: usize = *create_matches.get_one("page_size").expect("'page-size is required");

        },
        Some(("bench", bench_matches)) => {

        },
        _ => unreachable!("Magic"),
    }





    Ok(())
    //create()
}
