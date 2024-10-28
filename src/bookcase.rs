//use std::os::unix::prelude::OpenOptionsExt;
use std::{
    fs::{
        create_dir_all, remove_dir_all, File, OpenOptions
    }, path::{Path, PathBuf},
    os::unix::fs::OpenOptionsExt
};
use std::fmt;
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};

use super::PAGE_BYTES;
use log::trace;


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
    pub fn new(path_prefix: PathBuf,
               directory_prefix: String,
               directory_count: u64,
               file_prefix: String,
               file_count: u64,
               file_size: usize,
               ) -> Result<BookCasePlans> {

        if !path_prefix.exists() {
            return Err(anyhow!("check path prefix; does not exist: {}", path_prefix.display()));
        }
        if directory_count == 0 || (file_count / directory_count) < 1 {
            return Err(anyhow!("attempted to create more directories ({directory_count}) than files ({file_count})"));
        }
        if file_size < PAGE_BYTES || file_size.rem_euclid(PAGE_BYTES) != 0 {
            return Err(anyhow!("invalid file size; file size must be non-zero multiple of {PAGE_BYTES}"));
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
        trace!("Getting configuration from file {config_file_path}");

        use std::fs::read_to_string;
        let bookcase: BookCase = serde_json::from_str(&read_to_string(config_file_path)?)?;
        Ok(BookCasePlans {bookcase})
    }

    pub fn to_configuration_file(&self) -> Result<()> {
        use std::fs::write;
        trace!("Writing configuration to file $prefix_path/config.json");

        
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
        trace!("Creating BookCase file structure");

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
        let file: File = OpenOptions::new().read(read)
                          .write(write)
                          .create(
                               if read && !write { false }
                               else              { true  }
                           )
                          .custom_flags(
                              if self.direct_io { libc::O_DIRECT }
                              else              { 0 }
                          )
                          .open(&fpath)?;
        Ok(file)
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
    #[allow(unused_imports)]
    use serial_test::serial;

    /// Basic test for the creation and deletion of the BookCase file structure.
    /// This test uses a temporary dir that is deleted once the function scope
    /// is exited.
    #[test]
    #[serial]
    fn creation_and_destruction() {
        use tempfile::{tempdir, TempDir};
        use super::{BookCasePlans,BookCase};
        use super::super::PAGE_BYTES;
        
        let temp_dir: TempDir = tempdir().expect("Could not create temp directory for unit test");
        
        // FAILING TESTS
        //////////////////////////////////////////////////////////
        let test_vectors_pass: Vec<(u64, u64, usize)> = vec![
            (1,0,PAGE_BYTES * 1),
            (0,1,PAGE_BYTES * 1),
            (2,1,PAGE_BYTES * 1),
            (1,1,PAGE_BYTES * 0),
        ];
        test_vectors_pass.iter().for_each(|(dir_count, file_count, file_size)|{
            let fcount: u64 = *file_count;
            let fsize: usize = *file_size;
            let dcount: u64 = *dir_count;

            assert!(BookCasePlans::new(temp_dir.path().to_path_buf(),
                                       String::from("shelf"),
                                       dcount,
                                       String::from("book"),
                                       fcount,
                                       fsize).is_err());
        });


        // PASSING TESTS
        //////////////////////////////////////////////////////////
        let test_vectors_pass: Vec<(u64, u64, usize)> = vec![
            (1,1,PAGE_BYTES * 1),
            (1,10,PAGE_BYTES * 1),
            (1,100,PAGE_BYTES * 100),
            (100,100,PAGE_BYTES * 1024),
        ];

        test_vectors_pass.iter().for_each(|(dir_count, file_count, file_size)|{
            let fcount: u64 = *file_count;
            let fsize: usize = *file_size;
            let dcount: u64 = *dir_count;

            let bookcase_plan: BookCasePlans = BookCasePlans::new(temp_dir.path().to_path_buf(),
                                                                  String::from("shelf"),
                                                                  dcount,
                                                                  String::from("book"),
                                                                  fcount,
                                                                  fsize).expect("Could not create BookCasePlans");
            let bookcase: BookCase = bookcase_plan.construct().expect("Could not construct BookCase");

            assert!(bookcase.is_assembled());

            // Test the number and characteristics of created files.
            assert!(fcount == bookcase.book_count());
            (0..fcount).for_each(|fid| {
                let metadata = std::fs::metadata(bookcase.book_location(fid as u64)).expect("Unable to read BookCase file metadata");
                assert!(metadata.is_file());
                assert!(fsize == metadata.len() as usize);
                assert!(metadata.len() == bookcase.book_size() as u64);
            });
            
            // Test the number and characteristics of created directories.
            assert!(dcount == bookcase.shelf_count());
            (0..dcount).for_each(|did| {
                let metadata = std::fs::metadata(bookcase.shelf_location(did)).expect("Unable to read BookCase file metadata");
                assert!(metadata.is_dir());
            });

            // Tear down
            bookcase.deconstruct().expect("Unable to remove BookCase file structure");

            assert!(!bookcase.is_assembled());

            // Assert all artifacts removed
            (0..fcount).for_each(|fid| {
                assert!(std::fs::metadata(bookcase.book_location(fid)).is_err());
            });
            (0..dcount).for_each(|did| {
                assert!(std::fs::metadata(bookcase.shelf_location(did)).is_err());
            });
        });
    }
    
    #[test]
    #[serial]
    fn configuration_file() {
        unimplemented!("TODO: implement tests for the configuration file feature");
    }
}
