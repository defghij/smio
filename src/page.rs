use rand_xoshiro::rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoroshiro128PlusPlus;
use std::fmt;
use bytemuck::{
    Pod, Zeroable,
};


#[allow(unused_macros)]
macro_rules! assert_page_eq {
    ($A:expr, $B:expr, $W:expr) => {
        let mut assert_failed: bool = false;
        if $A.seed != $B.seed { 
            assert_failed = true;
            println!("Seed differs:\n\t0x{:016X}\n\t0x{:016X}", $A.seed, $B.seed);
        }
        if $A.file != $B.file { 
            assert_failed = true;
            println!("file differs:\n\t0x{:016X}\n\t0x{:016X}", $A.file, $B.file);
        }
        if $A.page != $B.page {
            assert_failed = true;
            println!("page differs:\n\t0x{:016X}\n\t0x{:016X}", $A.page, $B.page);
        }
        if $A.mutations != $B.mutations {
            assert_failed = true;
            println!("mutations differs:\n\t0x{:016X}\n\t0x{:016X}", $A.mutations, $B.mutations);
        }
        if $A.data != $B.data {
            assert_failed = true;
            println!("Data differs:");
            let mut number_of_differences: usize = 0;
            for i in 0..$W{
                let bytes_a = $A.data[i].to_be_bytes();
                let bytes_b = $B.data[i].to_be_bytes();
                if bytes_a != bytes_b {
                    println!("\tword: {i}");
                    println!("\t\t{:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}\n",
                           bytes_a[0], bytes_a[1], bytes_a[2], bytes_a[3],
                           bytes_a[4], bytes_a[5], bytes_a[6], bytes_a[7]);
                    println!("\t\t{:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}\n",
                           bytes_b[0], bytes_b[1], bytes_b[2], bytes_b[3],
                           bytes_b[4], bytes_b[5], bytes_b[6], bytes_b[7]);
                    number_of_differences += 1;

                }
                if number_of_differences >= 8 {
                    println!("More than 8 words of Data differ. Excluding remaining words...");
                }
            }
        }
        if assert_failed { panic!("Page comparison failed."); }
    };
}


/* TODO: 
 *  - WORDS should be words for the total structure. Not just the data segment. Dependent on
 *  `generic_const_exprs`. This is because I would like something like:
 *  ```Rust
 *  Page<PAGE_SIZE_IN_BYTES> which implies
 *      page.data has type [u64; (PAGE_SIZE_IN_BYTES / 64) - (METADATA_SIZE / 64)];
 *  ```
 */
/// A structure to ecapsulate meta-data and data derived there from. Metadata
/// is used to create a seed which is fed to a hashing function to generate
/// data. All elements are 8-byte aligned. Meta data can be mutated at which
/// point the data is updated accordingly.
///
/// # Metadata
/// Metadata determines the bytes contained in data. Metadata fields are as follows:
/// - seed: Base seed used for pages across entire application. 
/// - file: File in which this Page resides.
/// - page: The Page index into the file that refers to this instance.
/// - mutations: How many times this page has been mutated.
///
/// The four fields above result in a single, final, seed.
///
/// # Data
/// Data is generated using Xoroshiro128PlusPlus hashing function. The seed is
/// derived from the meta_data fields and the function is iterated to generate
/// the required number of words. Data contains `const W:usize` u64 words.
///
/// # Usage
/// This type is meant to be single, referable, data unit written to a file.
/// The position in file (page) and among files (file) as well as the base 
/// seed (seed) and how many times the Page has been altered create a
/// deterministic, pseudo-random, data which can be written to and read from
/// the file system.
#[repr(C, align(8))]
#[derive(Debug, Copy, Clone)]
pub struct Page<const W: usize> {
    seed: u64,
    file: u64,
    page: u64,
    mutations: u64,
    data: [u64; W]
} impl<const W: usize> Page<W> {
    pub const METADATA_WORDS: usize = 4;
    pub const METADATA_BYTES: usize = Self::METADATA_WORDS * std::mem::size_of::<u64>();
    pub const DATA_WORDS: usize = W;
    pub const DATA_BYTES: usize = W * std::mem::size_of::<u64>();
    pub const PAGE_WORDS: usize = Self::METADATA_WORDS + Self::DATA_WORDS;
    pub const PAGE_BYTES: usize = Self::PAGE_WORDS * std::mem::size_of::<u64>();

    /// Creates a new, populated, instace of Page.
    #[allow(dead_code)]
    pub fn new(seed: u64, file: u64, page: u64) -> Page<W> {
        let mutations: u64 = 0;
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(seed, file, page, mutations));
        Page::<W> { file, seed, page, mutations, data }
    }

    /// Creates an empty, zeroed, Page.
    pub fn default() -> Page<W> {
        Page::<W> {
            seed: 0,
            file: 0,
            page: 0,
            mutations: 0,
            data: [0u64; W]
        }
    }

    // TODO: Include mutations in all of the relevant functions below

    ////////////////////////////////////////////////////
    //// Data Functions

    /// Combine seed elements into a final seed suitable for passing to
    /// `self.generate_data` function.
    fn assemble_seed(seed: u64, file: u64, page: u64, mutations: u64) -> u64 {
        let seed_page: u64 = page << 32;   
        let seed_file: u64 = !(file) << 46;
        let seed_upper: u64 = seed_file | seed_page;
        let seed_lower: u64 = seed;
        let seed: u64 = seed_upper | seed_lower;
        seed + mutations
    }
   
    /// Invokes the hash function to generate data for Page.
    fn generate_data(seed: u64) -> [u64; W] {
        let mut rng = Xoroshiro128PlusPlus::seed_from_u64(seed);
        let data: [u64; W] = {
            let mut temp = [0; W];
            for elem in temp.iter_mut() {
                *elem = rng.next_u64();
            }
            temp
        };
        data
    }

    /// Will return true if supplied arguments result in data that is consistent 
    /// with self.data. This function will generate data from supplied arguments
    /// and compare to state of self.
    pub fn validate_page_with(self, seed: u64, file: u64, page: u64, mutations: u64) -> bool {
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(seed, file, page, mutations));
        data == self.data
    }

    /// This does uses the instatiated types meta-data to verify the 
    /// data words. Because the all elements of the meta-data are used
    /// in data word creation (hash), any corruption in either will
    /// lead to a negative (false) result.
    pub fn is_valid(&self) -> bool {
        self.validate_page_with(self.seed, self.file, self. page, self.mutations)
    }
     
    pub fn get_metadata(&self) -> (u64, u64, u64, u64) {
        (self.seed,
         self.file,
         self.page,
         self.mutations)

    }

    ////////////////////////////////////////////////////
    //// Mutatate/Transmute Functions
    /// All mutate functions cause the re-generation of the data contained in a page.

    /// Reinitialize the page. This function alters all parts of the Page metadata.
    /// This is the same as creating a new page except `mutations` must be provided.
    #[allow(dead_code)]
    pub fn reinit(&mut self, seed: u64, file: u64, page: u64, mutations: u64) -> &Self {
        self.seed = seed;
        self.file = file;
        self.page = page;
        self.mutations = mutations;
        self.data = Page::generate_data(Page::<W>::assemble_seed(self.seed, self.file, self.page, self.mutations));
        self
    }

    /// Advance mutation count by one. This generates new page data.
    #[allow(dead_code)]
    pub fn mutate(&mut self) -> &Self {
        self.mutations += 1;
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(self.seed, self.file, self.page, self.mutations));
        self.data = data;
        self
    }

    /// Advance mutation count by one. This generates new page data.
    #[allow(dead_code)]
    pub fn update_seed(&mut self, seed: u64) -> &Self {
        self.seed = seed;
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(self.seed, self.file, self.page, self.mutations));
        self.data = data;
        self
    }

    /// Alter the file meta data field. This generates new page data.
    #[allow(dead_code)]
    pub fn update_file(&mut self, file: u64) -> &Self {
        self.file = file;
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(self.seed, self.file, self.page, self.mutations));
        self.data = data;
        self
    }

    /// Alter the page meta data field. This generates new data.
    #[allow(dead_code)]
    pub fn update_page(&mut self, page: u64) -> &Self {
        self.page = page;
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(self.seed, self.file, self.page, self.mutations));
        self.data = data;
        self
    }
}

impl<const W:usize> PartialEq for Page<W> {
    /// Field by field equality test for the `Page<W>` type. Dependent
    /// on equality of all fields of the data type.
    fn eq(&self, other: &Self) -> bool {
        if self.seed != other.seed           { return false; }
        if self.file != other.file           { return false; }
        if self.page != other.page           { return false; }
        if self.mutations != other.mutations { return false; }
        if self.data != other.data           { return false; }
        true
    }
}

impl<const W:usize> fmt::Display for Page<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Seed:    0x{:016X}\n", self.seed)?;
        write!(f, "FileID:  0x{:016X}\n", self.file)?;
        write!(f, "PageID:  0x{:016X}\n", self.page)?;
        write!(f, "MutCnt:  0x{:016X}\n", self.mutations)?;
        write!(f, "Data:\n")?;
        for i in 0..W {
            let bytes = self.data[i].to_be_bytes();
            write!(f, "{:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}\n",
                   bytes[0], bytes[1], bytes[2], bytes[3],
                   bytes[4], bytes[5], bytes[6], bytes[7])?;
        }
        Ok(())
    }
}

// TODO:
// Justify these marker traits
unsafe impl<const W:usize> Pod for Page<W> {}
unsafe impl<const W:usize> Zeroable for Page<W> {
    fn zeroed() -> Self {
        Page::default()
    }
}

/// These tests confirm that general funcationality of the Page type,
/// primarily the Page<W> --> Page<W> functions, word as expected.

mod validation {

    #[test]
    fn seed_alterations() {
        use super::Page;
        let mut page: Page<1> = Page::new(0xdead+0xbeef, 1, 1);


        // Check that reinit generates the right Page data
        page.reinit(0xdead, 1, 1, 0);
        let bytes: [u64; 1] = Page::generate_data(Page::<1>::assemble_seed(0xdead, 1, 1, 0));
        assert!(page.data == bytes, "reinit failed");

        // Verify that mutate modifies the assembled seed correctly.
        page.mutate();
        let bytes: [u64; 1] = Page::generate_data(Page::<1>::assemble_seed(0xdead, 1, 1, 1));
        assert!(page.data == bytes, "mutate seed failed");

        
        page.update_file(0);
        let bytes: [u64; 1] = Page::generate_data(Page::<1>::assemble_seed(0xdead+1, 0, 1, 0));
        assert!(page.data == bytes, "mutate file failed");


        page.update_page(0);
        let bytes: [u64; 1] = Page::generate_data(Page::<1>::assemble_seed(0xdead+1, 0, 0, 0));
        assert!(page.data == bytes, "mutate page failed");


        assert!(Page::<0>::PAGE_BYTES == std::mem::size_of::<Page<0>>(), "{} != {}",Page::<0>::PAGE_BYTES, std::mem::size_of::<Page<0>>()) ;
        assert!(Page::<4096>::PAGE_BYTES == std::mem::size_of::<Page<4096>>(), "{} != {}",Page::<4096>::PAGE_BYTES, std::mem::size_of::<Page<4096>>()) ;
    }

    #[test]
    fn random_page_bytes_vec() {
        const W: usize = (512 / 8) - 4; // 512 words / 8 bytes per word - 4 metadata words
        const PAGE_COUNT: usize = 64;
        use super::Page;
        use rand::prelude::*;


        let mut rng: ThreadRng = rand::thread_rng();

        let pages: Vec<Page<W>> = (0..PAGE_COUNT).map(|i|{
                Page::new(rng.gen(), rng.gen(), i as u64)
            }).collect();
        
        
        for (_, page) in pages.iter().enumerate() {
            assert!(page.is_valid());
        }
    }
}
