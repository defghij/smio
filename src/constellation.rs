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
    root: u64
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
/// The goal of a Constellation is to instantiate a large number of files, potentially across file
/// systems, and to provide a very light-weight interface for accessing those files: open for
/// (read|write).
///
/// Interactions with the layout, after creation, are limited to individual files and some
/// inspection with respect to the dimension of the constellation.
///
/// # Constellation Structure
///
/// A constellation structure is composed of _roots_, _directories_ and _files_. A _root_ is a path
/// to a collection of _directories_. A _directory_ is a collection of _files_. Roots may span file
/// systems. 
///
/// Directories and Files will have the naming scheme: <prefix><ID>.
///
/// Directories and files will be created, and numbered (ID), in a round-robin fashion. That is
/// the first file will reside in the first directory, the second file in the second directory, and
/// so on. If their are more files than directories, they will loop back (as in modular
/// arithmetic). The directories and files are organized such that the file resides in the 
/// directory in which its ID is congruent. 
///
/// An example of a constellation:
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConstellation {
    /// Root path(s) for the directories
    roots: Vec<PathBuf>,

    /// Info about The directories under the root
    directories: LayerInfo,

    /// Info for files contained in a single directory
    files: LayerInfo,

    /// Whether this structure should remove the files and directories when dropped. 
    drop: bool,
} 
impl FileConstellation {

    /// TODO 
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

    /// Open a file with identifier `id` for (read|write). The `id` is the absolute identification
    /// of the file.
    #[inline(always)]
    pub fn open(&self, id: u64, read: bool, write: bool) -> Result<File> {
        self.open_with_checked_id(self.file_identifier(id)?, read, write)
    }

    /// Toggles whether Drop removes files and directories from the file system.
    pub fn toggle_drop(&mut self) { self.drop = !self.drop; }

    /// Returns the dimensions of the Constellation:
    ///     (roots, directories (per root), files (per directory))
    #[inline(always)]
    pub fn dimensions(&self) -> (u64, u64, u64) { 
        (self.roots.len() as u64, self.directories.count, self.files.count)
    }
    
    /// Returns the total number of files contained in the constellation. Convenience function,
    /// this is the same as the product of the return values from `dimension()`.
    #[inline(always)]
    pub fn count(&self) -> u64 {
        self.roots.len() as u64 * self.directories.count * self.files.count 
    }

    /// Returns the size of a file(s), in bytes, contained in the constellation
    #[inline(always)]
    pub fn size(&self) -> u64 { self.files.size.expect("files should always be declared with a size") }

    #[inline(always)]
    fn file_identifier(&self, id: u64) -> Result<FileIdentifier> {
        let (roots, directories, files): (u64, u64, u64) = self.dimensions();

        if (roots * directories * files) < id {
            Err(anyhow!("Requested file id is out of bounds: requested {} > {} max", id, self.files.count))
        } else {
            let file: u64      = id % (files * directories * roots);
            let directory: u64 = id % (        directories * roots);
            let root: u64      = id % (                      roots);

            Ok(FileIdentifier { file, directory, root })
        }
    }

    fn instantiate(fss: FileConstellation) -> Result<FileConstellation> {
        let (roots, directories, files): (u64, u64, u64) = fss.dimensions();
        let total_directories: u64 = roots * directories;
        let total_files: u64 = total_directories * files;

        // Create all directories
        (0..total_directories)
            .into_iter()
            .try_for_each(|d|{ fss.create_directory(fss.file_identifier(d)?) })?;

        // Create all files
        (0..total_files)
            .into_iter()
            .try_for_each(|f|{ fss.create_file(fss.file_identifier(f)?) })?;
        Ok(fss)
    }

    fn destroy(&self) -> Result<(), anyhow::Error> {
        let (roots, directories, files): (u64, u64, u64) = self.dimensions();
        let total_directories: u64 = roots * directories;
        let total_files: u64 = total_directories * files;

        // Remove all files
        (0..total_files)
             .into_iter()
             .try_for_each(|f|{
                 self.remove_file(self.file_identifier(f)?)
             })?;

        // Remove all directories
        (0..total_directories)
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
        let (roots, directories, files): (u64, u64, u64) = self.dimensions();

        let dwidth: usize = ((roots * directories        ).ilog10() + 1) as usize;
        let fwidth: usize = ((roots * directories * files).ilog10() + 1) as usize;

        let mut location: PathBuf = self.roots[id.root as usize].clone();

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
        let (roots, directories, files): (u64, u64, u64) = self.dimensions();

        let dwidth: usize = ((roots * directories        ).ilog10() + 1) as usize;
        let fwidth: usize = ((roots * directories * files).ilog10() + 1) as usize;
        //let root_id = id.file % self.roots.len() as u64;

        //write!(f, "Roots:........... {:?}\n", self.root
                                                  //.to_str()
                                                  //.expect("Undefined FileConstellation root"))?;
        writeln!(f, "Roots:............")?;
        self.roots.iter().try_for_each(|r| {
            writeln!(f, "\t{}", r.display())
        })?;

        writeln!(f, "Directory Names:.. {}[{:0width$}-{:0width$}]\n", 
                    self.directories.prefix,
                    0,
                    (roots * directories) - 1,
                    width = dwidth)?;

        writeln!(f, "File Names:....... {}[{:0width$}-{:0width$}]\n", 
                    self.files.prefix,
                    0,
                    (roots * directories * files) - 1,
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
        
        let (r, d, f) = files.dimensions();

        // Test the number and characteristics of created files.
        let mut total_files: u64 = 0;
        (0..(r*d*f))
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
        assert_eq!(total_files,(r*d*f));

        println!("{files}");

        // destroy is used by drop so this ensures Drop trait is functional
        files.destroy().expect("Unable to destroy FileConstellation");
    }
}
