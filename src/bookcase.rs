//use std::os::unix::prelude::OpenOptionsExt;
use std::{
    fs::{
        create_dir_all, remove_dir_all, remove_file, File, OpenOptions
    }, path::{Path, PathBuf},
    os::unix::fs::OpenOptionsExt
};
use std::io::Result;
use std::sync::Arc;
use std::fmt;
use serde::{Deserialize, Serialize};

// I dont know why this complains about unused import
#[allow(unused_imports)]
use serial_test::serial;


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
    directory_prefix: String,
    directory_count: u64,
    file_prefix: String,
    file_count: u64,
    page_size: usize,
    page_count: u64,
    direct_io: bool,
    pub seed: u64,
} impl BookCase {
    pub fn new(path_prefix: PathBuf,
               directory_prefix: String,
               directory_count: u64,
               file_prefix: String,
               file_count: u64,
               page_size: usize,
               page_count: u64,
               seed: u64
               ) -> BookCase {

        if !path_prefix.exists() {
            panic!("Path to root does not exist");
        }

        BookCase {
            constructed: false,
            path_prefix: path_prefix,
            directory_prefix: directory_prefix,
            directory_count,
            file_prefix: file_prefix,
            file_count,
            page_size,
            page_count,
            direct_io: false,
            seed
        }
    }

    pub fn from_string(path: &str) -> BookCase {
        use std::fs::read_to_string;

        let data: String = read_to_string(path).expect("Unable to read configuration file");
        serde_json::from_str(&data).expect("Unable to deserialize data from configuration file")
    }

    pub fn write_configuration_file(&self) {
        use std::fs::write;
        let serialized = serde_json::to_string_pretty(self).unwrap();
        write(format!("{}/config.json", self.path_prefix.to_str().unwrap()), serialized).expect("Unable to write configuration file");
    }

    /// Creates the directory (Shelf) and file (Book) structure
    /// described by the instantiation of this type.
    pub fn construct(&mut self) -> Result<()> {
        if !self.constructed {
            for fid in 0..(self.file_count as usize) {
                self.create_book(fid as u64)?;
            }
            self.constructed = true;
        }
        Ok(())
    }

    /// Removes the directory (Shelf) and file (Book) structure
    /// described by the instantiation of this type.
    pub fn demolish(&mut self) -> Result<()> {
        if self.constructed {
            for id in 0..(self.directory_count as usize) {
                remove_dir_all(self.shelf_location(id as u64).to_str().unwrap())?;
            }
            self.constructed = false;
        }
        Ok(())
    }

    fn create_book(&self, file_id: u64) -> Result<()> {
        let fsize: usize = self.book_size();
        let path: PathBuf = self.book_location(file_id);
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

    #[allow(dead_code)]
    fn destroy_book(self, id: u64) -> Result<()> {
        let fpath: String = self.book_location(id).to_str().unwrap().to_string();
        remove_file(&fpath)
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
        assert!(id < self.file_count);

        let mut book_location: PathBuf = self.shelf_location(id % self.directory_count);
        let book: String = format!("{}{:0fwidth$}",
                                    self.file_prefix,
                                    id,
                                    fwidth = (self.file_count.ilog10() + 1) as usize);
        book_location.push(book);
        book_location
    }
    
    pub fn shelf_location(&self, id:u64) -> PathBuf {
        assert!(id < self.directory_count);
        let mut location: PathBuf = PathBuf::new();
        location.push(self.path_prefix.to_str().unwrap());

        let shelf: String = format!("{}{:0dwidth$}", 
                                    self.directory_prefix, 
                                    id.rem_euclid(self.directory_count),
                                    dwidth = (self.directory_count.ilog10() + 1) as usize);
        location.push(shelf);
        location
    }

    #[inline(always)]
    pub fn book_size(&self) -> usize {
        (self.page_count as usize) * self.page_size
    }

    #[inline(always)]
    pub fn shelf_count(&self) -> u64 {
        self.directory_count
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
        let dwidth: usize = (self.directory_count.ilog10() + 1) as usize;
        let fwidth: usize = (self.file_count.ilog10() + 1) as usize;
        write!(f, "Root:............ {:?}\n", self.path_prefix.to_str().unwrap())?;
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

/****************************
 * File system Tests need
 * to be ran serially. This is because if multiple
 * tests call construct/demolish, they may interfere
 * with other tests that expect files to be present.
 */

#[test]
#[serial]
fn creation_and_demolition() {
    use std::fs::{read_dir, remove_dir};
    use tempfile::tempdir;

    let test_dir = tempdir().unwrap().into_path();

    println!("[Info] Test attempt to create and destory a collection of files in '{}'", test_dir.to_str().unwrap());
    let pprefix: PathBuf = test_dir.clone();
    let dprefix: String = String::from("cad-shelf");
    let fprefix: String = String::from("book");
    let mut bookcase: BookCase = BookCase::new(
                          pprefix.clone(),
                          dprefix,
                          2,
                          fprefix,
                          4,
                          512,
                          2,
                          0xDeadBeef);

    bookcase.construct().expect("Could not create test bookcase structures.");

    // Get all file names in the test directories.
    let files: Vec<String> = read_dir(test_dir.as_path())
                                    .expect("unable to read directory")
                                    .into_iter()
                                    .flat_map(|d| {
                                        let files: Vec<String> = read_dir(d.unwrap().path())
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
    assert!(files.len() == bookcase.book_count() as usize);

    (0..bookcase.book_count()).into_iter()
                              .for_each(|f|{
                                  assert!(files.contains(&bookcase.book_location(f)
                                                                  .to_str().unwrap()
                                                                  .to_string()))
                              });

    bookcase.demolish().expect("Could not create test bookcase structures.");
    let files: Vec<String> = read_dir(test_dir.as_path())
                                    .expect("unable to read directory")
                                    .into_iter()
                                    .flat_map(|d| {
                                        let files: Vec<String> = read_dir(d.unwrap().path())
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
    assert!(files.len() == 0);
    remove_dir(pprefix.as_path()).unwrap();
}


#[test]
#[serial]
fn create_open_destroy_book() {
    // fn create_book(&self, file_id: u64) -> Result<()> {
    // fn destroy_book(self, id: u64) -> Result<()> {
    // pub fn open_book(&self, id: u64, read: bool, write: bool) -> File {
    use std::io::ErrorKind;
    use tempfile::tempdir;

    let test_dir = tempdir().unwrap().into_path();

    println!("[Info] Test attempt to create and destory a collection of files in '{}'", test_dir.to_str().unwrap());
    
    let pprefix: PathBuf = test_dir;
    let dprefix: String = String::from("codb-shelf");
    let fprefix: String = String::from("book");
    let bookcase: BookCase = BookCase::new(
                          pprefix.clone(),
                          dprefix,
                          1,
                          fprefix,
                          1,
                          512,
                          1,
                          0xDeadBeef);

    assert!(bookcase.open_book(0, true, false).err().unwrap().kind() == ErrorKind::NotFound);
    assert!(bookcase.open_book(0, false, true).err().unwrap().kind() == ErrorKind::NotFound);
    assert!(bookcase.open_book(0, true, true).err().unwrap().kind() == ErrorKind::NotFound);
    
    bookcase.clone().construct().expect("Could not construct bookcase");

    assert!(bookcase.open_book(0, true, false).is_ok());
    assert!(bookcase.open_book(0, false, true).is_ok());
    assert!(bookcase.open_book(0, true, true).is_ok());

    bookcase.clone().destroy_book(0).expect("Could not destroy book");

    // implicitly creates book then opens.
    assert!(bookcase.open_book(0, false, true).is_ok());
    bookcase.clone().destroy_book(0).expect("Could not destroy book");



    // TODO: This should not be an error-- bookcase.demolish should clean up the shelves
    bookcase.clone().demolish().expect("Could not construct bookcase");
    assert!(std::fs::remove_dir(pprefix.as_path()).is_err());  //.err().unwrap().kind() == ErrorKind::DirectoryNotEmpty);
    
    assert!(std::fs::remove_dir_all(pprefix.as_path()).is_ok());
}

