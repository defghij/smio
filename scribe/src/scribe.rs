use std::fs::{
    File,
    OpenOptions
};
//use std::os::unix::prelude::OpenOptionsExt;
use std::sync::atomic::AtomicUsize;
use std::{path::Path, fs};
use std::io::{
    Result,
    Write
};
use crate::page::Page;


const MAX_DIRECTORIES: usize = 10000;
const PAGE_SIZE:       usize = 4096; 
const PAGE_SIZE_MIN:   usize = 4096;
const CHAPTER_SIZE:    usize = 512;
const PAGES_PER_FILE: usize = 512;
//const O_DIRECT: i32 = 0x4000;
const DSEGSIZE:       usize = 8;  

fn create_file_with_size(path: &str, size: usize) -> Result<()> {
    let path: &Path = Path::new(path);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file: File = File::create(path)?;

    if file.metadata()?.len() != size as u64 {
        file.set_len(size as u64)?;
    }
    Ok(())
}

fn create_directories_and_files(path_prefix: &String,
                                directory_prefix: &String,
                                directory_count: usize,
                                file_prefix: &String,
                                file_count: usize,
                                file_size: usize,
                                verbose: bool) -> Result<()> {
    for file_id in 0..file_count {
        let file_path: String = format!("{}/{}{}/{}{}",
                                    path_prefix,
                                    directory_prefix,
                                    file_id.rem_euclid(directory_count),
                                    file_prefix,
                                    file_id
                                );
        let result = create_file_with_size(&file_path, file_size);
        match result {
            Ok(_) => if verbose { println!("Created: {}", file_path) },
            Err(why) => panic!("[E] Unable to create file ({}): {}", file_path, why),
        }
    }
    Ok(())
}

pub fn create() -> Result<()> {
    /********************************
     * Create Directories and files *
     ********************************/
    let prefix: String = String::from("/home/chuck/programming/testing");
    let dir_prefix: String = String::from("dir");
    let file_prefix: String = String::from("file");

    let file_count: usize = 6;
    let dir_count: usize = 3;

    let page_size: usize = 4 + 8 + 4 + 8 * DSEGSIZE;
    let file_size: usize = page_size * PAGES_PER_FILE; 
    let _ = create_directories_and_files(&prefix,
                                         &dir_prefix,
                                         dir_count,
                                         &file_prefix,
                                         file_count,
                                         file_size,
                                         false);

    let amount_of_work: AtomicUsize = AtomicUsize::new(file_count * PAGES_PER_FILE);


    /**************************
     * Create and write pages *
     **************************/
    // Initialize to a default
    let book: &mut [Page<DSEGSIZE>; PAGES_PER_FILE] = &mut [Page::<DSEGSIZE>::new(0, 0, 0); PAGES_PER_FILE];

    for file_id in 0..file_count {
        let file_path: String = format!("{}/{}{}/{}{}",
                                    prefix,
                                    dir_prefix,
                                    file_id.rem_euclid(dir_count),
                                    file_prefix,
                                    file_id
                                );

        for page_id in 0..PAGES_PER_FILE {
            let p: Page<DSEGSIZE> = Page::new(file_id as u32, page_id as u64, 0xdead);
            book[page_id] = p;
        }

        let deflated: Vec<u8> = match bitcode::encode(book) {
            Ok(d) => d,
            Err(why) => panic!("[E] Unable to serialize pages: {}", why)
        };

        let mut file = OpenOptions::new()
                        .write(true)
                        //.custom_flags(O_DIRECT)
                        .create(true)
                        .open(&file_path)
                        .expect("[E] Failed to open file for writing!");

        file.write_all(&deflated).expect("[E] Failed to write book to disk!");
        file.flush()?;
    }
    Ok(())
}



