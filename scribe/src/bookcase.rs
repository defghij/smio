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
    constructed: bool,
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
            constructed: false,
            path_prefix: Arc::new(path_prefix),
            directory_prefix: Arc::new(directory_prefix),
            directory_count,
            file_prefix: Arc::new(file_prefix),
            file_count,
            page_size,
            page_count
        }
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
                let dpath: String = format!("{}/{}{:0width$}",
                    self.path_prefix,
                    self.directory_prefix,
                    id,
                    width = (self.directory_count.ilog10() + 1) as usize );
                remove_dir_all(dpath)?;
            }
            self.constructed = false;
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

    pub fn open_book(&self, id: u64, read: bool, write: bool) -> Result<File> {
        let fpath: String = self.book_location(id);
        OpenOptions::new()
                    .read(read)
                    .write(write)
                    //.custom_flags(O_DIRECT)
                    .create({
                        if read && !write { false }
                        else              { true  }
                    })
                    .open(&fpath)
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

#[test]
fn create_open_destroy_book() {
    // fn create_book(&self, file_id: u64) -> Result<()> {
    // fn destroy_book(self, id: u64) -> Result<()> {
    // pub fn open_book(&self, id: u64, read: bool, write: bool) -> File {
    use std::io::ErrorKind;
    
    let pprefix: String = String::from("./testing");
    let dprefix: String = String::from("shelf");
    let fprefix: String = String::from("book");
    let mut bookcase: BookCase = BookCase::new(
                          pprefix.to_owned(),
                          dprefix.to_owned(),
                          1,
                          fprefix.to_owned(),
                          1,
                          512,
                          1);

    assert!(bookcase.open_book(0, true, false).err().unwrap().kind() == ErrorKind::NotFound);
    assert!(bookcase.open_book(0, false, true).err().unwrap().kind() == ErrorKind::NotFound);
    
    bookcase.clone().construct().expect("Could not construct bookcase");

    assert!(bookcase.open_book(0, true, false).is_ok());
    assert!(bookcase.open_book(0, false, true).is_ok());

    bookcase.clone().destroy_book(0).expect("Could not destroy book");

    // FIXME: Why is this None instead of Err(_)?
    assert!(bookcase.open_book(0, true, false).err().unwrap().kind() == ErrorKind::NotFound);
    assert!(bookcase.open_book(0, false, true).err().unwrap().kind() == ErrorKind::NotFound);

    bookcase.demolish().unwrap();
    std::fs::remove_dir(pprefix).unwrap();
}

fn creation_and_demolition() {
    use std::fs::{read_dir, remove_dir};

    let pprefix: String = String::from("./testing");
    let dprefix: String = String::from("shelf");
    let fprefix: String = String::from("book");
    let mut bookcase: BookCase = BookCase::new(
                          pprefix.to_owned(),
                          dprefix.to_owned(),
                          2,
                          fprefix.to_owned(),
                          4,
                          512,
                          2);

    bookcase.construct().expect("Could not create test bookcase structures.");

    // Get all file names in the test directories.
    let files: Vec<String> = read_dir("./testing")
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
                                  assert!(files.contains(&bookcase.book_location(f)))
                              });

    bookcase.demolish().expect("Could not create test bookcase structures.");
    let files: Vec<String> = read_dir("./testing")
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
    remove_dir(pprefix).unwrap();
}
