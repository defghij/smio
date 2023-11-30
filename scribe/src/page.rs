use rand_xoshiro::rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoroshiro128PlusPlus;
use std::fmt;
//use serde::{Serialize, Deserialize};
use bytemuck::{
    Pod, Zeroable,
    //try_from_bytes, bytes_of,
};

pub const METADATA_SIZE: usize = 32 /*bytes*/;

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
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Page<const W: usize> {
    seed: u64,
    file: u64,
    page: u64,
    mutations: u64,
    data: [u64; W]
} impl<const W: usize> Page<W> {
    #[allow(dead_code)]
    pub fn new(seed: u64, file: u64, page: u64) -> Page<W> {
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(seed, file, page));
        Page::<W> {
            file,
            seed,
            page,
            mutations: 0,
            data
        }
    }

    pub fn default() -> Page<W> {
        Page::<W> {
            file: 0,
            seed: 0,
            page: 0,
            mutations: 0,
            data: [0u64; W]
        }
    }

    
    ////////////////////////////////////////////////////
    //// Data Functions

    /// TODO: Why does this function require a generic argument?
    fn assemble_seed(seed: u64, file: u64, page: u64) -> u64 {
        let seed_page: u64 = page << 32;   
        let seed_file: u64 = !(file) << 46;
        let seed_upper: u64 = seed_file | seed_page;
        let seed_lower: u64 = seed;
        let seed: u64 = seed_upper | seed_lower;
        seed
    }
    
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

    pub fn validate_page_with(self, seed: u64, file: u64, page: u64) -> bool {
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(seed, file, page));
        data == self.data
    }

    ////////////////////////////////////////////////////
    //// Mutatate/Transmute Functions
    #[allow(dead_code)]
    pub fn reinit(&mut self, seed: u64, file: u64, page: u64, mutations: u64) -> &Self {
        self.seed = seed;
        self.file = file;
        self.page = page;
        self.mutations = mutations;
        self.data = Page::generate_data(Page::<W>::assemble_seed(self.seed + self.mutations, self.file, self.page));
        self
    }

    #[allow(dead_code)]
    pub fn mutate_seed(&mut self) -> &Self {
        self.seed += 1;
        self.mutations += 1;
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(self.seed, self.file, self.page));
        self.data = data;
        self
    }

    #[allow(dead_code)]
    pub fn mutate_file(&mut self, file: u64) -> &Self {
        self.file = file;
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(self.seed, self.file, self.page));
        self.data = data;
        self
    }

    #[allow(dead_code)]
    pub fn mutate_page(&mut self, page: u64) -> &Self {
        self.page = page;
        let data: [u64; W] = Page::generate_data(Page::<W>::assemble_seed(self.seed, self.file, self.page));
        self.data = data;
        self
    }
}

impl<const W:usize> PartialEq for Page<W> {
    fn eq(&self, other: &Self) -> bool {
        if self.seed != other.seed { return false; }
        if self.file != other.file { return false; }
        if self.page != other.page { return false; }
        if self.mutations != other.mutations { return false; }
        if self.data != other.data { return false; }
        true
    }
}

/// &Page<W> --> &[u8]
impl<'a,const W:usize> TryFrom<&'a Page<W>> for &'a [u8] {
    type Error = bytemuck::PodCastError;
    fn try_from(value: &'a Page<W>) -> Result<Self, Self::Error> {
        Ok(bytemuck::bytes_of(value))
    }
}

/// Slice conversion: &[u8] --> &Page<W>
impl<'a,const W:usize> TryFrom<&'a [u8]> for &'a Page<W> {
    type Error = bytemuck::PodCastError;
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        bytemuck::try_from_bytes(value)
    }
}

/// Array conversion: &[u8;N] --> &Page<W>
impl<'a,const N: usize, const W:usize> TryFrom<&'a [u8;N]> for &'a Page<W> {
    type Error = bytemuck::PodCastError;
    fn try_from(value: &'a [u8; N]) -> Result<Self, Self::Error> {
        bytemuck::try_from_bytes(value)
    }
}

/// Vector conversion: &Vec<u8> --> &Page<W>
impl<'a,const W:usize> TryFrom<&'a Vec<u8>> for &'a Page<W> {
    type Error = bytemuck::PodCastError;
    fn try_from(value: &'a Vec<u8>) -> Result<Self, Self::Error> {
        bytemuck::try_from_bytes(value)
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

#[test]
fn general_functionality() {
    let mut page: Page<1> = Page::new(0xdead+0xbeef, 1, 1);


    page.reinit(0xdead, 1, 1, 0);
    let bytes: [u64; 1] = Page::generate_data(Page::<1>::assemble_seed(0xdead, 1, 1));
    assert!(page.data == bytes, "reinit failed");


    page.mutate_seed();
    let bytes: [u64; 1] = Page::generate_data(Page::<1>::assemble_seed(0xdead+1, 1, 1));
    assert!(page.data == bytes, "mutate seed failed");

    
    page.mutate_file(0);
    let bytes: [u64; 1] = Page::generate_data(Page::<1>::assemble_seed(0xdead+1, 0, 1));
    assert!(page.data == bytes, "mutate file failed");


    page.mutate_page(0);
    let bytes: [u64; 1] = Page::generate_data(Page::<1>::assemble_seed(0xdead+1, 0, 0));
    assert!(page.data == bytes, "mutate page failed");
}


#[allow(dead_code)]
/// These tests serve both as correctness tests and 
/// as explorations into different methods to convert between types.
/// In the latter way they are correct, though perhaps not idomatic,
/// ways to convert between different types of interest.
/// 
/// There are three categories of tests: to bytes, from bytes, and both ways.
/// In the final case, this relies on some intermediate type or structure (i.e
/// the file system).
mod transmutation {
    pub const S: u64  = 0xD7D6D5D4D3D2D1D0;
    pub const F: u64  = 0xC7C6C5C4C3C2C1C0;
    pub const P: u64  = 0xB7B6B5B4B3B2B1B0;
    pub const M: u64  = 0x0000000000000000;
    pub const D1: u64 = 0xAFDF3EC403080884;
    pub const D2: u64 = 0xD127816C6EF096AB;

    mod to_u8 {
        /// Test different ways of converting from Page<Words> to [u8]
        #[test]
        fn single_page() {
            use super::{
                S, F, P,
                super::{
                    Page,
                    METADATA_SIZE,
                }
            };
            use bytemuck;
            let flat_tv: [u8; 40]  = [
                0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7,  // seed
                0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,  // file
                0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,  // page
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // mutations
                0x84, 0x08, 0x08, 0x03, 0xC4, 0x3E, 0xDF, 0xAF,  //  data cont'd
            ];
            const W: usize = 1;
            const PAGE_SIZE: usize = METADATA_SIZE + W * 8;

            // &Page<W> --> &[u8]
            let page: Page<W> = Page::new(S, F, P);
            let bytes: &[u8] = bytemuck::bytes_of(&page);
            assert!(&flat_tv == bytes);

            // Box<Page<W>> --> &[u8]
            let page: Page<W> = Page::new(S, F, P);
            let page_box: Box<Page<W>> = Box::new(page);
            let bytes: &[u8] = bytemuck::bytes_of(page_box.as_ref());
            assert!(&flat_tv == bytes.as_ref());

            // test that the pointers for Box<Page<W>> --> &[u8] are the same.
            let page_box_ptr: *const Page<W> = &*page_box;
            let bytes_ptr: *const [u8] = &*bytes;
            assert!(format!("{page_box_ptr:?}") == format!("{bytes_ptr:?}"), "Pointers are not equal: {page_box_ptr:?} != {bytes_ptr:?}");
           

            // Box<Page<W>> --> &[u8]
            let bytes: &[u8] = page_box.as_ref().try_into().expect("Unable to convert");
            assert!(&flat_tv == bytes);
        }

        #[test]
        fn two_pages() {
            use super::{
                S, F, P,
                super::{
                    Page,
                    METADATA_SIZE,
                }
            };
            let flat_tv  = vec![
                // Page One
                0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7,  // seed
                0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,  // file
                0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,  // page
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // mutations
                0x84, 0x08, 0x08, 0x03, 0xC4, 0x3E, 0xDF, 0xAF,  //  data
                // Page Two
                0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7,  // seed
                0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,  // file
                0xB1, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,  // page
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // mutations
                0xAB, 0x96, 0xF0, 0x6E, 0x6C, 0x81, 0x27, 0xD1,  //  data
            ];
            const W: usize = 1;
            const PAGE_SIZE: usize = METADATA_SIZE + W * 8;
            const PAGES: usize = 2;


            // &[Page<Words>; 2] --> &[u8]
            let page1: Page<W> = Page::new(S, F, P);
            let page2: Page<W> = Page::new(S, F, P + 1);
            let pages: [Page<W>; 2] = [page1, page2];

            let bytes: &[u8] = bytemuck::bytes_of(&pages);
            assert!(flat_tv == bytes);

            
            // [Page<W>; PAGES] --> Vec<u8>
            let bytes: Vec<u8> = pages.iter().flat_map(|p| {
                let bytes: &[u8] = p.try_into().expect("Unable to convert");
                bytes.to_vec()
            }).collect();
            assert!(flat_tv == bytes);


            // Box<[Page<W>]> --> &[u8]
            let pages: Box<[Page<W>]> = Box::new([
                Page::new(S, F, P), Page::new(S, F, P+1)
            ]);
            let bytes: &[u8] = bytemuck::bytes_of(TryInto::<&[Page<W>;PAGES]>::try_into(pages.as_ref()).expect("Unable to convert"));
            assert!(flat_tv == bytes);
            

            // Vec<Page<W>> --> Vec<u8>
            let mut pages: Vec<Page<W>> = Vec::with_capacity(PAGES);
            pages.push(Page::new(S, F, P));
            pages.push(Page::new(S, F, P+1));
           
            let bytes: Vec<u8> = pages.clone()
                                      .into_boxed_slice()
                                      .iter()
                                      .flat_map(|p| {
                                          let pbytes: &[u8] = p.try_into().expect("Unable to convert");
                                          pbytes.to_vec()
                                      })
                                      .collect();
            assert!(flat_tv == bytes);
            
            // Vec<Page<W>> --> Vec<u8>
            let bytes: Vec<u8> = pages.into_iter()
                                      .map(|p |{ <&Page<W> as TryInto<&[u8]>>::try_into(&p).expect("Unable to convert").to_vec() })
                                      .flatten()
                                      .collect();
            assert!(flat_tv == bytes);
        }
    }

    mod from_u8 {
        #[test]
        fn single_page() {
            use super::{
                S, F, P,
                super::Page
            };
            let flat_tv: [u8; 40]  = [
                0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7,  // seed
                0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,  // file
                0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,  // page
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // mutations
                0x84, 0x08, 0x08, 0x03, 0xC4, 0x3E, 0xDF, 0xAF,  //  data cont'd
            ];
            const W: usize = 1;
            let page_tv: Page<W> = Page::new(S, F, P);

            // Test different single page transformations
            let pages: [&Page<W>; 4] = [
                bytemuck::try_from_bytes(&flat_tv)
                       .expect("Unable to convert bytes to Page!"),
                flat_tv.as_slice()
                       .try_into()
                       .expect("Unable to convert bytes to Page"),
                flat_tv.as_ref()
                       .try_into()
                       .expect("Unable to convert bytes to Page"),
                (&flat_tv).try_into()
                          .expect("Unable to convert bytes to Page")
            ];

            for page in pages.iter() {
                assert!(*page == &page_tv);
            }
        }

        #[test]
        fn two_pages() {
            use super::{
                S, F, P,
                super::Page
            };
            let flat_tv  = vec![
                // Page One
                0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7,  // seed
                0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,  // file
                0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,  // page
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // mutations
                0x84, 0x08, 0x08, 0x03, 0xC4, 0x3E, 0xDF, 0xAF,  //  data
                // Page Two
                0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7,  // seed
                0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,  // file
                0xB1, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,  // page
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // mutations
                0xAB, 0x96, 0xF0, 0x6E, 0x6C, 0x81, 0x27, 0xD1,  //  data
            ];

            const W: usize = 1;
            const PAGES: usize = 2;
            let pages_tv: [Page<W>; PAGES] = [
                Page::new(S, F, P),
                Page::new(S, F, P+1)
            ];

            // Vec<u8> --> Vec<Page<W>>
            let pages: Vec<Page<W>> = bytemuck::try_from_bytes::<[Page<W>; PAGES]>(&flat_tv)
                                                        .expect("Could not convert bytes to Page!")
                                                        .to_vec();
            assert!(*pages == pages_tv);


            // Vec<u8> --> &[Page<W>]
            let pages: &[Page<W>] = bytemuck::try_from_bytes::<[Page<W>; PAGES]>(&flat_tv)
                                                        .expect("Could not convert bytes to Page!");
            assert!(*pages == pages_tv);
            

            // Vec<u8> --> &[Page<W>; PAGES]
            let pages: &[Page<W>; PAGES] = bytemuck::try_from_bytes::<[Page<W>; PAGES]>(&flat_tv)
                                                        .expect("Could not convert bytes to Page!");
            assert!(*pages == pages_tv);


            // &[u8] --> &[Page<W>; PAGES]
            let flat_tv: &[u8] = flat_tv.as_slice();
            let pages: &[Page<W>;PAGES] = bytemuck::try_from_bytes(flat_tv)
                                                .expect("Could not convert bytes to Page!");
            assert!(*pages == pages_tv);
        }
    }


    mod to_and_from {
        #[test]
        fn random_page_bytes_array() {
            use super::super::{
                Page,
                METADATA_SIZE,
                super::memory_ops::{
                    to_byte_slice,
                    from_byte_slice
                }
            };
            use rand::prelude::*;
            use array_init::array_init;

            const PAGE_SIZE: usize = 512;
            const PAGE_COUNT: usize = 64;
            const W: usize = (PAGE_SIZE - (METADATA_SIZE)) / 8;

            let mut rng: ThreadRng = rand::thread_rng();

            let seed: u64 = rng.gen();
            let file: u64 = rng.gen();

            let pages: [Page<W>; PAGE_COUNT] = array_init(|i: usize| Page::new(seed, file, i as u64));
            let pages_bytes: &[u8; PAGE_COUNT * PAGE_SIZE] = to_byte_slice(&pages);

            let pages: &[Page<W>; PAGE_COUNT] = from_byte_slice(pages_bytes).expect("Could not transmute page!");

            for (p, page) in pages.iter().enumerate() {
                assert!(page.validate_page_with(seed, file, p as u64)); 
            }
        }

        #[test]
        fn random_page_bytes_vec() {
            use super::super::{
                Page,
                METADATA_SIZE,
            };
            use rand::prelude::*;
            //use array_init::array_init;

            const PAGE_SIZE: usize = 512;
            const PAGE_COUNT: usize = 64;
            const W: usize = (PAGE_SIZE - (METADATA_SIZE)) / 8;

            let mut rng: ThreadRng = rand::thread_rng();

            let seed: u64 = rng.gen();
            let file: u64 = rng.gen();

            let pages: Vec<Page<W>> = (0..PAGE_COUNT).map(|i|{
                    Page::new(seed, file, i as u64)
                }).collect();
            
            
            for (p, page) in pages.iter().enumerate() {
                assert!(page.validate_page_with(seed, file, p as u64)); 
            }
        }
        #[test]
        fn vec_writes_and_reads() {
            use super::super::{
                Page,
                METADATA_SIZE,
            };
            use rand::prelude::*;
            use std::{
                fs::File,
                io::Write,
            };

            const PAGE_SIZE: usize = 512;
            const PAGE_COUNT: usize = 64;
            const W: usize = (PAGE_SIZE - (METADATA_SIZE)) / 8;
            let tmpfile_name: String = String::from("test.page.serde");
            let mut tmpfile: File = File::create(tmpfile_name.clone()).expect("Was not able to create temporary file!");

            let mut rng: ThreadRng = rand::thread_rng();

            let seed: u64 = rng.gen();
            let file: u64 = rng.gen();

            let pages: Vec<Page<W>> = (0..PAGE_COUNT).map(|i|{
                    Page::new(seed, file, i as u64)
                }).collect();

            let write_buffer: Vec<u8> = pages.into_iter()
                                             .map(|p |{ <&Page<W> as TryInto<&[u8]>>::try_into(&p).expect("Unable to convert").to_vec() })
                                             .flatten()
                                             .collect();

            // Transition from bits in address-space to bits in file-space
            tmpfile.write_all(write_buffer.as_slice()).unwrap();
            tmpfile.flush().expect("Could not flush temporary file");
            drop(tmpfile); // Let OS/Rust reap this file descriptor.
        
            let read_buffer: Vec<u8> =  std::fs::read(tmpfile_name.clone()).expect("Could not read file");
            if read_buffer.len() != PAGE_SIZE * PAGE_COUNT {
                std::fs::remove_file(tmpfile_name.clone()).expect("Unable to remove temporary testing file");
                assert!(read_buffer.len() == PAGE_SIZE * PAGE_COUNT, "Read {} of {} bytes", read_buffer.len(), PAGE_SIZE * PAGE_COUNT);
            }

            let pages_w: &[Page<W>; PAGE_COUNT] = bytemuck::try_from_bytes::<[Page<W>; PAGE_COUNT]>(&read_buffer.as_slice())
                                                        .expect("Could not convert bytes to Page");

            for (p, page) in pages_w.iter().enumerate() {
                if !page.validate_page_with(seed, file, p as u64) {
                    std::fs::remove_file(tmpfile_name.clone()).expect("Unable to remove temporary testing file");
                    assert!(false, "Failed to valid page {} of {}", p, PAGE_COUNT);
                }
            }
            std::fs::remove_file(tmpfile_name.clone()).expect("Unable to remove temporary testing file");
        }
    }
}
