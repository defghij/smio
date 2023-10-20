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
use std::fmt;
use crate::page::Page;


const MAX_DIRECTORIES: usize = 10000;
pub const PAGE_SIZE: usize = 4096; 
pub const METADATA_SIZE: usize = 16;
pub const DATA_SIZE: usize     = PAGE_SIZE - METADATA_SIZE;
const PAGE_SIZE_MIN:   usize = 4096;
const CHAPTER_SIZE:    usize = 512;
pub const PAGE_COUNT: u64 = 512;
//const O_DIRECT: i32 = 0x4000;
const DSEGSIZE:       usize = 8;  



/*******************************
 *
 * What Needs To Happen
 * - Create Mode:
 *  - Func :: Create directories
 *  - Func :: Create Files (truncated)
 *  - Spawn Threads
 *    - Loop
 *    - Thread pulls work
 *    - Opens File
 *    - Generates Pages
 *      - When queue threshold reached, write
 *      - When write threshold reached, write (may be same as above)
 *      - When end of file, write
 *
 * Scribe should handle only the I/O adjacent tasks
 */

#[derive(Debug, Clone, Copy)]
pub struct BookCase<'a> {
    path_prefix: &'a str,
    directory_prefix: &'a str,
    directory_count: u32,
    file_prefix: &'a str,
    file_count: u32,
    page_size: usize,
    page_count: u64,
} impl<'a> BookCase<'a> {
    pub fn new(path_prefix: &'a str,
               directory_prefix: &'a str,
               directory_count: u32,
               file_prefix: &'a str,
               file_count: u32,
               page_size: usize,
               page_count: u64
               ) -> BookCase<'a> {

        BookCase {
            path_prefix,
            directory_prefix,
            directory_count,
            file_prefix,
            file_count,
            page_size,
            page_count
        }
    }

    pub fn build(self) -> Result<()> {
        for fid in 0..(self.file_count as usize) {
            self.create_book(fid as u32)?;
        }
        Ok(())
    }

    pub fn demolish(self) -> Result<()> {
        for fid in 0..(self.file_count as usize) {
            self.destroy_book(fid as u32)?;
        }
        Ok(())
    }

    fn create_book(self, file_id: u32) -> Result<()> {
        let fsize: usize = self.book_size();
        let path: String = self.book_location(file_id);
        let path: &Path  = Path::new(&path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file: File = File::create(path)?;

        if file.metadata()?.len() != fsize as u64 {
            file.set_len(fsize as u64).expect("Couldnt set file length!");
        }
        Ok(())
    }

    fn destroy_book(self, file_id: u32) -> Result<()> {
        unimplemented!("Not yet!");
    }

    pub fn open_book(self, file_id: u32, read: bool, write: bool) -> File {
        let fpath: String = self.book_location(file_id);
        OpenOptions::new()
                    .write(write)
                    .read(read)
                    //.custom_flags(O_DIRECT)
                    .create(true)
                    .open(&fpath)
                    .expect("[E] Failed to open file for writing!")
    }

    ////////////////////////////////////////////////////
    //// Utility Functions
    #[inline(always)]
    pub fn book_location(self, file_id: u32) -> String {
        format!("{}/{}{}/{}{:0width$}",
            self.path_prefix,
            self.directory_prefix,
            file_id.rem_euclid(self.directory_count),
            self.file_prefix,
            file_id,
            width = (self.file_count.ilog10() + 1) as usize )
    }

    #[inline(always)]
    pub fn book_size(self) -> usize {
        (self.page_count as usize) * self.page_size
    }

    #[inline(always)]
    pub fn book_count(self) -> u32 {
        self.file_count
    }

    #[inline(always)]
    pub fn page_count(self) -> u64 {
        self.page_count
    }
} impl<'a> fmt::Display for BookCase<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dwidth: usize = (self.directory_count.ilog10() + 1) as usize;
        let fwidth: usize = (self.file_count.ilog10() + 1) as usize;
        write!(f, "Root:............ {}\n", self.path_prefix)?;
        write!(f, "Directory Name:.. {}[{:0width$}-{:0width$}]\n", 
                    self.directory_prefix,
                    0,
                    self.directory_count - 1,
                    width = dwidth)?;
        write!(f, "File Name:....... {}[{:0width$}-{:0width$}]\n", 
                    self.file_prefix,
                    0,
                    self.file_count - 1,
                    width = fwidth)?;
        write!(f, "Page Size:....... {}\n", self.page_size)?;
        write!(f, "Page Count:...... {}\n", self.page_count)?;
        Ok(())
    }
}




/*
pub fn open_book(path:String) -> File {


    let amount_of_work: AtomicUsize = AtomicUsize::new(file_count * PAGES_PER_FILE);


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


        file.write_all(&deflated).expect("[E] Failed to write book to disk!");
        file.flush()?;
    }
    Ok(())
}*/



