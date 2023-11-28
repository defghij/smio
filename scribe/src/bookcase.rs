//use std::os::unix::prelude::OpenOptionsExt;
use std::{
    path::Path,
    fs::{
        OpenOptions,
        File,
        remove_file,
        remove_dir_all,
        create_dir_all,
    }
};
use std::io::Result;
use std::sync::Arc;
use std::fmt;


/// A structure that encapsulates everything the application could
/// know about the directory, file, and page structure at run-time
/// as well as relevant operations on that structure.
///
/// This application has numerous side-effects as it interacts with
/// the operating system to create and desctory inodes.
#[derive(Debug, Clone)]
pub struct BookCase {
    path_prefix: Arc<String>,
    directory_prefix: Arc<String>,
    directory_count: u64,
    file_prefix: Arc<String>,
    file_count: u64,
    page_size: usize,
    page_count: u64,
} impl BookCase {
    pub fn new(path_prefix: String,
               directory_prefix: String,
               directory_count: u64,
               file_prefix: String,
               file_count: u64,
               page_size: usize,
               page_count: u64
               ) -> BookCase {

        BookCase {
            path_prefix: Arc::new(path_prefix),
            directory_prefix: Arc::new(directory_prefix),
            directory_count,
            file_prefix: Arc::new(file_prefix),
            file_count,
            page_size,
            page_count
        }
    }

    pub fn construct(&self) -> Result<()> {
        for fid in 0..(self.file_count as usize) {
            self.create_book(fid as u64)?;
        }
        Ok(())
    }

    pub fn demolish(&self) -> Result<()> {
        for id in 0..(self.directory_count as usize) {
            let dpath: String = format!("{}/{}{:0width$}",
                self.path_prefix,
                self.directory_prefix,
                id,
                width = (self.directory_count.ilog10() + 1) as usize );
            remove_dir_all(dpath)?;
        }
        Ok(())
    }

    fn create_book(&self, file_id: u64) -> Result<()> {
        let fsize: usize = self.book_size();
        let path: String = self.book_location(file_id);
        let path: &Path  = Path::new(&path);

        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let file: File = File::create(path)?;

        if file.metadata()?.len() != fsize as u64 {
            file.set_len(fsize as u64).expect("Couldnt set file length!");
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn destroy_book(self, id: u64) -> Result<()> {
        let fpath: String = self.book_location(id);
        remove_file(&fpath)
    }

    pub fn open_book(&self, id: u64, read: bool, write: bool) -> File {
        let fpath: String = self.book_location(id);
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
    pub fn book_location(&self, id: u64) -> String {
        assert!(id < self.file_count);
        format!("{}/{}{:0dwidth$}/{}{:0fwidth$}",
            self.path_prefix,
            self.directory_prefix,
            id.rem_euclid(self.directory_count),
            self.file_prefix,
            id,
            dwidth = (self.directory_count.ilog10() + 1) as usize,
            fwidth = (self.file_count.ilog10() + 1) as usize )
    }

    #[inline(always)]
    pub fn book_size(&self) -> usize {
        (self.page_count as usize) * self.page_size
    }

    #[inline(always)]
    pub fn book_count(&self) -> u64 {
        self.file_count
    }

    #[inline(always)]
    pub fn page_count(&self) -> u64 {
        self.page_count
    }

    #[inline(always)]
    pub fn word_count(&self) -> u64 {
        self.page_size as u64 / 8
    }

} impl fmt::Display for BookCase {
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
