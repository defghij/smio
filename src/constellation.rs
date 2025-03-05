//use std::os::unix::prelude::OpenOptionsExt;
use std::{
    fs::{
        File, OpenOptions
    }, path::PathBuf,
    os::unix::fs::OpenOptionsExt
};
use std::fmt;
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};
    

//use log::trace;
//use log::debug;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOptions {
    pub directo_io: bool,
}

#[derive(Debug, Clone)]
struct FileIdentifier {
    file: u64,
    directory: u64,
    root: usize
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayerInfo {
    /// String prefix for the layer's name.
    prefix: String,
    /// Total number of items in this layer
    count: u64,
    /// The number of bytes in items of this layer (only relevant for files)
    size: Option<u64>,
    /// Additional options for this layer (only relevant for files)
    options: Option<FileOptions>,
}

/// # Overview 
/// This structure and its associated functions are used to create, interact with, and then remove
/// a file layout or constellation. 
///
/// Interactions with the layout, after creation, are limited to `open_for_reading` and
/// `open_for_writing` to a specified _absolute_ file identification number.
///
/// # Constellation Structure
///
/// A constellation will a structure similar to the following:
///
/// ```txt
/// FileConstellation
/// |_ /sda   (root 0)
/// |  |- /dir0       // 0 = FileId % DirCount
/// |  |  |- /file00  
/// |  |  |- /file04
/// |  |  `- /file08
/// |  `- /dir2       // 2 = FileId % DirCount
/// |     |- /file02
/// |     |- /file06
/// |     `- /file10
/// `- /sdb   (root 1)     
///    |- /dir1       // 1 = FileId % DirCount
///    |  |- /file01
///    |  |- /file05
///    |  `- /file09
///    `- /dir3       // 3 = FileId % DirCount
///       |- /file03
///       |- /file07
///       `- /file11
/// ```
///
/// Note that under the `root`s  directories and files that differ with respect to their peers only
/// in the number. Each is composed of a `prefix` and a `number` such that we get
/// `<prefix><number>` for each. 
///
/// The directories and files are organized such that the file resides
/// in the directory in which its id is congruent. 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConstellation {
    /// Root path for the directories
    roots: Vec<PathBuf>,

    /// Info about The directories under the root
    directories: LayerInfo,

    /// Info for files contained in a single directory
    files: LayerInfo,

    /// Whether this structure should respect drop semantics 
    drop: bool,
} 
impl FileConstellation {
    pub fn new(roots: Vec<PathBuf>, 
               directories_per_root: (String, u64),
               files_per_directory: (String, u64), 
               size_of_files: u64, 
               options: FileOptions,
               drop: bool) -> Result<FileConstellation> {

        // Validation of parameters
        roots.iter().try_for_each(|root| {
            if root.exists() { Ok(()) } 
            else {
                Err(anyhow!("check path prefix; does not exist: {}", root.display()))
            }
        })?;

        if directories_per_root.1 == 0 || (files_per_directory.1 / directories_per_root.1) < 1 {
            return Err(anyhow!("attempted to create more directories ({}) than files ({})", 
                    directories_per_root.1, files_per_directory.1));
        }

        if size_of_files == 0  {
            return Err(anyhow!("invalid file size; file size must be non-zero"));
        }

        // NOTE: Is this really a constraint?
        if size_of_files % 64 != 0  {
            return Err(anyhow!("invalid file byte alignment; file size must be a multiple of 64"));
        }

        // Set up configuration
        let file_system_structure: FileConstellation =  FileConstellation {
                roots,
                directories: LayerInfo { prefix: directories_per_root.0,
                                         count: directories_per_root.1,
                                         size: None,
                                         options: None
                },
                files:       LayerInfo { prefix: files_per_directory.0,
                                         count: files_per_directory.1,
                                         size: Some(size_of_files),
                                         options: Some(options)
                },
                drop,
        };
        
        FileConstellation::instantiate(file_system_structure)
    }

    pub fn from_configuration(_file: &PathBuf) -> Result<FileConstellation> {
        unimplemented!("future feature");
    }

    pub fn open(&self, absolute_id: u64, read: bool, write: bool) -> Result<File> {
        self.open_with_checked_id(self.file_identifier(absolute_id)?, read, write)
    }

    /// Inverts the flag that indicates whether Drop removes files from the file system.
    pub fn toggle_drop(&mut self) { self.drop = !self.drop; }
    
    /// Returns the total number of files contained in the constellation
    pub fn count(&self) -> u64 { self.files.count * self.directories.count }

    /// Returns the size of a file(s), in bytes, contained in the constellation
    pub fn size(&self) -> u64 { self.files.size.expect("files should always be declared with a size") }

    #[inline(always)]
    fn file_identifier(&self, absolute_id: u64) -> Result<FileIdentifier> {
        if (self.directories.count * self.files.count) < absolute_id {
            Err(anyhow!("Requested file id is out of bounds: requested {} > {} max", absolute_id, self.files.count))
        } else {
            let file: u64      = absolute_id % (self.files.count * self.directories.count);
            let directory: u64 = absolute_id % self.directories.count;
            let root: usize    = absolute_id as usize % self.roots.len();

            Ok(FileIdentifier { file, directory, root })
        }
    }

    fn instantiate(fss: FileConstellation) -> Result<FileConstellation> {
        // Create all directories
        (0..fss.directories.count)
            .into_iter()
            .try_for_each(|d|{ fss.create_directory(fss.file_identifier(d)?) })?;

        // Create all files
        (0..(fss.files.count * fss.directories.count))
            .into_iter()
            .try_for_each(|f|{ fss.create_file(fss.file_identifier(f)?) })?;
        Ok(fss)
    }

    fn destroy(&self) -> Result<(), anyhow::Error> {
        // Remove all files
        (0..(self.files.count * self.directories.count))
             .into_iter()
             .try_for_each(|f|{
                 self.remove_file(self.file_identifier(f)?)
             })?;

        // Remove all directories
        (0..self.directories.count)
            .into_iter()
            .try_for_each(|d|{
                self.remove_directory(self.file_identifier(d)?)
            })?;
        Ok(())
    }

    #[inline(always)]
    fn open_with_checked_id(&self, id: FileIdentifier, read: bool, write: bool) -> Result<File> {
        let path: PathBuf = self.construct_path(id)?;

        let direct_io: bool = self.files.options.clone().is_some_and(|o| o.directo_io);
        let file: File = OpenOptions::new().read(read)
                          .write(write)
                          .create(
                               if read && !write { false }
                               else              { true  }
                           )
                          .custom_flags(
                              if direct_io { libc::O_DIRECT }
                              else { 0 }
                          )
                          .open(&path)?;
        Ok(file)
    }

    #[inline(always)]
    fn construct_path(&self, id: FileIdentifier) -> Result<PathBuf> {
        let dwidth: usize = (self.directories.count.ilog10() + 1) as usize;
        let fwidth: usize = ((self.directories.count * self.files.count).ilog10() + 1) as usize;

        let mut location: PathBuf = self.roots[id.root].clone();

        let directory: String = format!("{}{:0width$}", self.directories.prefix, id.directory, width=dwidth);
        location.push(directory);

        let file: String = format!("{}{:0width$}", self.files.prefix, id.file, width=fwidth);
        location.push(file);

        Ok(location)
    }

    fn create_file(&self, id: FileIdentifier) -> Result<(), anyhow::Error> {
        let path: PathBuf = self.construct_path(id)?;
        if path.parent().is_some_and(|p| p.exists()) {
            OpenOptions::new()
                        .create(true)
                        .write(true)
                        //.custom_flags(
                        //    if self.direct_io { libc::O_DIRECT }
                        //    else              { 0 }
                        //)
                        .truncate(false)
                        .open(path)?
                        .set_len(self.files.size.unwrap_or_default())?;
            Ok(())
        } else {
            Err(anyhow!("attempted to create file but parent directory did not exist"))
        }
    }

    fn create_directory(&self, id: FileIdentifier) -> Result<(), anyhow::Error> {
         let mut path = self.construct_path(id)?;
         path.pop(); // pop off the file (last element)
         std::fs::create_dir(path)?;
         Ok(())
    }

    fn remove_file(&self, id: FileIdentifier) -> Result<(), anyhow::Error> {
        let path: PathBuf = self.construct_path(id)?;
        std::fs::remove_file(path)?;
        Ok(())
    }

    fn remove_directory(&self, id: FileIdentifier) -> Result<(), anyhow::Error> {
        let mut path: PathBuf = self.construct_path(id)?;
        path.pop(); // drop file off the end of the buffer
        std::fs::remove_dir(path)?;
        Ok(())
    }

}
impl Drop for FileConstellation {
    fn drop(&mut self) {
        if self.drop {
            match self.destroy() {
                Ok(_) => {},
                Err(e) => eprintln!("{}",e),
            };
        }
    }
}
impl fmt::Display for FileConstellation {
    /// TODO: Add printout for options, sizes, etc
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dwidth: usize = (self.directories.count.ilog10() + 1) as usize;
        let fwidth: usize = (self.files.count.ilog10() + 1) as usize;
        //let root_id = id.file % self.roots.len() as u64;

        //write!(f, "Roots:........... {:?}\n", self.root
                                                  //.to_str()
                                                  //.expect("Undefined FileConstellation root"))?;
        write!(f, "Roots:............")?;
        self.roots.iter().try_for_each(|r| {
            write!(f, "\t{}", r.display())
        })?;

        write!(f, "Roots:............")?;
        write!(f, "Directory Names:.. {}[{:0width$}-{:0width$}]\n", 
                    self.directories.prefix,
                    0,
                    self.directories.count - 1,
                    width = dwidth)?;

        write!(f, "File Names:....... {}[{:0width$}-{:0width$}]\n", 
                    self.files.prefix,
                    0,
                    self.files.count - 1,
                    width = fwidth)?;
        Ok(())
    }
}

pub mod tests {
    #[allow(unused)]
    use super::*;
    #[allow(unused)]
    use serial_test::serial;
    #[allow(unused)]
    use std::io::Write;

    #[test]
    #[serial]
    fn create_and_destroy() {
        use tempfile::tempdir;

        let fsize: u64 = 1024;
        let fcount: u64 = 12;
        let dcount: u64 = 12;

        let root_a = tempdir().expect("crate should be able to create temporary directories");
        let root_b = tempdir().expect("crate should be able to create temporary directories");
        let mut files: FileConstellation = FileConstellation::new(
            vec![root_a.into_path(),root_b.into_path()],
            ("test_dir".to_string(), dcount),
            ("test_file".to_string(),fcount),
            fsize,
            FileOptions { directo_io: false },
            false
        ).expect("created directories and files");

        files.toggle_drop();

        // Test the number and characteristics of created files.
        let mut total_files: u64 = 0;
        (0..(fcount*dcount))
            .for_each(|fid| {
                let file: File = files.open(fid, false, true)
                                      .expect("files created at constellation instantiation");
                match file.metadata() {
                    Ok(m) => {
                        assert!(m.is_file());
                        assert_eq!(m.len(),fsize);
                    },
                    Err(_) => panic!("Failed to get meta-data for file"),
                }
                total_files += 1;
            });
        assert_eq!(total_files,(fcount * dcount));

        println!("{files}");

        // destroy is used by drop so this ensures Drop trait is functional
        files.destroy().expect("Unable to destroy FileConstellation");
    }
}
