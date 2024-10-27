//use std::os::unix::prelude::OpenOptionsExt;
use std::{
    fs::{
        create_dir_all, remove_dir_all, File, OpenOptions
    }, path::{Path, PathBuf},
    os::unix::fs::OpenOptionsExt
};
use std::io::Result;
use std::fmt;
use serde::{Deserialize, Serialize};

// I dont know why this complains about unused import
#[allow(unused_imports)]
use serial_test::serial;


#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayerInfo {
    prefix: String,
    count: u64,
    size: usize
}

/// This type represents an as yet to be instantiated or constructed BookCase.
/// A BookCasePlans can be constructed by function call, _new_, or by
/// configuration file, _from_configuration file_.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookCasePlans {
    bookcase: BookCase
} impl BookCasePlans {

    /// Creates a new bookcase constructor type BookCasePlans
    /// from function arguments.
    ///
    /// ***Panic***: This function panics in the event that the `prefix_path`
    /// does not exist. This is an unrecoverable error as the prefix path is 
    /// how all files are located. If this is invalid then we cannot create
    /// a valid file structure.
    pub fn new(path_prefix: PathBuf,
               directory_prefix: String,
               directory_count: u64,
               file_prefix: String,
               file_count: u64,
               file_size: usize,
               ) -> Result<BookCasePlans> {

        if !path_prefix.exists() {
            panic!("check path prefix; does not exist");
        }

        Ok(
            BookCasePlans{ 
                bookcase: BookCase {
                    constructed: false,
                    path_prefix,
                    directory: LayerInfo { prefix: directory_prefix, count: directory_count, size: 0},
                    file:      LayerInfo { prefix: file_prefix,      count: file_count,      size: file_size},
                    direct_io: false,
                }
            }
        )
    }

    pub fn from_configuration_file(config_file_path: &str) -> Result<BookCasePlans> {
        use std::fs::read_to_string;
        let bookcase: BookCase = serde_json::from_str(&read_to_string(config_file_path)?)?;
        Ok(BookCasePlans {bookcase})
    }

    pub fn to_configuration_file(&self) -> Result<()> {
        use std::fs::write;
        let serialized = serde_json::to_string_pretty(self)?;
        let path = format!("{}/config.json", self.bookcase.path_prefix.to_str().unwrap()); 
        Ok(write(path, serialized)?)
    }

    fn create_book(&self, file_id: u64) -> Result<()> {
        let fsize: usize = self.bookcase.file.size;
        let path: PathBuf = self.bookcase.book_location(file_id);
        let path: &Path  = path.as_path();

        if let Some(parent) = path.parent() {
            //println!("Creating dir: {}", path.parent().unwrap().to_str().unwrap());
            create_dir_all(parent)?;
        }
        let file: File = File::create(path)?;

        if file.metadata()?.len() != fsize as u64 {
            file.set_len(fsize as u64).expect("Couldnt set file length!");
        }
        Ok(())
    }

    /// Creates, on the file system, the directory (Shelf) and
    /// file (Book) structure described by the instantiation of this type.
    /// This simply returned a previously constructed `BookCase` if one was
    /// previously created using this structure.
    pub fn construct(self) -> Result<BookCase> {
        if !self.bookcase.is_assembled() {
            for fid in 0..(self.bookcase.file.count as usize) {
                self.create_book(fid as u64)?;
            }
        } 
        Ok(self.bookcase)
    }

}


/// A structure that encapsulates everything the application could
/// know about the directory, file, and page structure at run-time
/// as well as relevant operations on that structure.
///
/// This application has numerous side-effects as it interacts with
/// the operating system to create and desctory inodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookCase {
    constructed: bool,
    path_prefix: PathBuf,
    directory: LayerInfo,
    file: LayerInfo,
    direct_io: bool,
} impl BookCase {

    /// Removes the directory (Shelf) and file (Book) structure
    /// described by the instantiation of this type.
    pub fn deconstruct(&self) -> Result<BookCasePlans> {
        if self.is_assembled() {
            for id in 0..(self.directory.count as usize) {
                remove_dir_all(self.shelf_location(id as u64).to_str().unwrap())?;
            }
        } 
        Ok(BookCasePlans {bookcase: self.to_owned()} )
    }

    pub fn open_book(&self, id: u64, read: bool, write: bool) -> Result<File> {
        let fpath: String = self.book_location(id).to_str().unwrap().to_string();
        OpenOptions::new().read(read)
                          .write(write)
                          .create(
                               if read && !write { false }
                               else              { true  }
                           )
                          .custom_flags(
                              if self.direct_io { libc::O_DIRECT }
                              else              { 0 }
                          )
                          .open(&fpath)
    }

    ////////////////////////////////////////////////////
    //// Configuration Functions
    pub fn use_direct_io(&mut self) {
        self.direct_io = true;
    }
    

    ////////////////////////////////////////////////////
    //// Structure Utility Functions
    #[inline(always)]
    pub fn book_location(&self, id: u64) -> PathBuf {
        assert!(id < self.file.count);

        let mut book_location: PathBuf = self.shelf_location(id % self.directory.count);
        let book: String = format!("{}{:0fwidth$}",
                                    self.file.prefix,
                                    id,
                                    fwidth = (self.file.count.ilog10() + 1) as usize);
        book_location.push(book);
        book_location
    }
    
    pub fn shelf_location(&self, id:u64) -> PathBuf {
        assert!(id < self.directory.count);
        let mut location: PathBuf = PathBuf::new();
        location.push(self.path_prefix.to_str().unwrap());

        let shelf: String = format!("{}{:0dwidth$}", 
                                    self.directory.prefix, 
                                    id.rem_euclid(self.directory.count),
                                    dwidth = (self.directory.count.ilog10() + 1) as usize);
        location.push(shelf);
        location
    }

    #[inline(always)]
    pub fn shelf_count(&self) -> u64 {
        self.directory.count
    }

    #[inline(always)]
    pub fn book_size(&self) -> usize {
        self.file.size
    }

    #[inline(always)]
    pub fn book_count(&self) -> u64 {
        self.file.count
    }

    /// This function walks the file system checking the whether the file structure
    /// expected by this ` BookCase` exists. If the structure does exist then it is 
    /// assumed the `BookCase` is assembled. Likewise, if there is not a matching file
    /// structure then it is assumed the `BookCase` is not assembled.
    ///
    /// ***Note***: This could return true even though the files have been removed. 
    /// Consider copying or otherwise backing up the `BookCase` file structure prior to
    /// it being removed by this application. Then after remove it is copied back. This
    /// is a degenerate case not handled here.
    pub fn is_assembled(&self) -> bool {
        use std::fs::read_dir;

        // Get all file names in the test directories.
        let files: Vec<String> = read_dir(self.path_prefix.clone())
                                        .expect("unable to read directory")
                                        .into_iter()
                                        .filter(|dir| {
                                            dir.as_ref().unwrap().metadata().unwrap().is_dir()
                                        })
                                        .flat_map(|dir| {
                                            let files: Vec<String> = read_dir(dir.unwrap().path())
                                                                            .expect("Unable to read sub dir")
                                                                            .into_iter()
                                                                            .map(|f| { f.unwrap()
                                                                                       .path()
                                                                                       .into_os_string()
                                                                                       .into_string()
                                                                                       .unwrap()
                                                                            })
                                                                            .collect();
                                            files
                                        })
                                        .collect();


        // The two following tests ensure that:
        //      - |files| == |books|
        //      - books is subset of files
        //      - if x and y are in books then x != y.
        // which implies they are identical.
        if files.len() != self.book_count() as usize { return false; }

        // True if all books accounted for otherwise false.
        !(0..self.book_count()).into_iter()
                               .map(|f|{
                                  !files.contains(&self.book_location(f)
                                                       .to_str().unwrap()
                                                       .to_string())
                               })
                               .fold(false, |acc, x| acc && x)
    }

} impl fmt::Display for BookCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dwidth: usize = (self.directory.count.ilog10() + 1) as usize;
        let fwidth: usize = (self.file.count.ilog10() + 1) as usize;
        write!(f, "Root:............ {:?}\n", self.path_prefix.to_str().unwrap())?;
        write!(f, "Directory Name:.. {}[{:0width$}-{:0width$}]\n", 
                    self.directory.prefix,
                    0,
                    self.directory.count - 1,
                    width = dwidth)?;
        write!(f, "File Name:....... {}[{:0width$}-{:0width$}]\n", 
                    self.file.prefix,
                    0,
                    self.file.count - 1,
                    width = fwidth)?;
        Ok(())
    }
}


// TODO: Should test creation and destruction
mod testing {

}
