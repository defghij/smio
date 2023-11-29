use rand_xoshiro::rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoroshiro128PlusPlus;
use std::fmt;
//use serde::{Serialize, Deserialize};
use bytemuck::{
    Pod, Zeroable,
    //try_from_bytes, bytes_of,
};



//TODO
//-----------------------
// 1. Need to be able to write pages an array in memory? Maybe the stack is sufficient?
                              
pub const METADATA_SIZE: usize = 32 /*bytes*/;
use super::PAGE_SIZE;

/* TODO: 
 *  - Add a mutation count parameter that adds to the base preseed to yield new data
 *  - WORDS should be words for the total structure. Not just the data segment
 */ 
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Page<const WORDS: usize> {
    seed: u64,
    file: u64,
    page: u64,
    mutations: u64,
    data: [u64; WORDS]
} impl<const WORDS: usize> Page<WORDS> {
    #[allow(dead_code)]
    pub fn new(seed: u64, file: u64, page: u64) -> Page<WORDS> {
        let data: [u64; WORDS] = Page::generate_data(Page::<WORDS>::assemble_seed(seed, file, page, 0));
        Page::<WORDS> {
            file,
            seed,
            page,
            mutations: 0,
            data
        }
    }

    pub fn default() -> Page<WORDS> {
        Page::<WORDS> {
            file: 0,
            seed: 0,
            page: 0,
            mutations: 0,
            data: [0u64; WORDS]
        }
    }

    
    ////////////////////////////////////////////////////
    //// Data Functions
    #[allow(dead_code)]
    fn assemble_seed(seed: u64, file: u64, page: u64, mutations: u64) -> u64 {
        let seed_page: u64 = page << 32;   
        let seed_file: u64 = !(file) << 46;
        let seed_upper: u64 = seed_file | seed_page;
        let seed_lower: u64 = seed + mutations;
        let seed: u64 = seed_upper | seed_lower;
        seed
    }
    
    fn generate_data(seed: u64) -> [u64; WORDS] {
        let mut rng = Xoroshiro128PlusPlus::seed_from_u64(seed);
        let data: [u64; WORDS] = {
            let mut temp = [0; WORDS];
            for elem in temp.iter_mut() {
                *elem = rng.next_u64();
            }
            temp
        };
        data
    }

    pub fn validate_page_with(self, seed: u64, file: u64, page: u64, mutations: u64) -> bool {
        let data: [u64; WORDS] = Page::generate_data(Page::<0>::assemble_seed(seed, file, page, mutations));
        data == self.data
    }

    ////////////////////////////////////////////////////
    //// Mutatate/Transmute Functions
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let len = std::mem::size_of::<Page<WORDS>>();
            std::slice::from_raw_parts(self as *const Page<WORDS> as *const u8, len)
        }
    }
    pub fn from_bytes(bytes: &[u8; PAGE_SIZE]) -> &Page<WORDS> {
        unsafe {
            std::mem::transmute::<&[u8;PAGE_SIZE], &Page<WORDS>>(bytes)
        }

    }
    #[allow(dead_code)]
    pub fn reinit(&mut self, seed: u64, file: u64, page: u64, mutations: u64) {
        self.seed = seed;
        self.file = file;
        self.page = page;
        self.mutations = mutations;
        self.data = Page::generate_data(Page::<WORDS>::assemble_seed(seed, file, page, mutations));
    }

    #[allow(dead_code)]
    pub fn mutate_seed(&mut self) {
        self.seed += 1;
        self.mutations += 1;
        let data: [u64; WORDS] = Page::generate_data(Page::<0>::assemble_seed(self.seed, self.file, self.page, self.mutations));
        self.data = data;
    }

    #[allow(dead_code)]
    pub fn mutate_file(&mut self, file: u64) {
        self.file = file;
        let data: [u64; WORDS] = Page::generate_data(Page::<1>::assemble_seed(self.seed, self.file, self.page, self.mutations));
        self.data = data;
    }

    #[allow(dead_code)]
    pub fn mutate_page(&mut self, page: u64) {
        self.page = page;
        let data: [u64; WORDS] = Page::generate_data(Page::<1>::assemble_seed(self.seed, self.file, self.page, self.mutations));
        self.data = data;
    }
}

impl<const WORDS:usize> PartialEq for Page<WORDS> {
    fn eq(&self, other: &Self) -> bool {
        if self.seed != other.seed { return false; }
        if self.file != other.file { return false; }
        if self.page != other.page { return false; }
        if self.mutations != other.mutations { return false; }
        if self.data != other.data { return false; }
        true
    }
}

/// &Page<WORDS> --> &[u8]
impl<'a,const WORDS:usize> TryFrom<&'a Page<WORDS>> for &'a [u8] {
    type Error = bytemuck::PodCastError;
    fn try_from(value: &'a Page<WORDS>) -> Result<Self, Self::Error> {
        Ok(bytemuck::bytes_of(value))
    }
}

/// Slice conversion: &[u8] --> &Page<WORDS>
impl<'a,const WORDS:usize> TryFrom<&'a [u8]> for &'a Page<WORDS> {
    type Error = bytemuck::PodCastError;
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        bytemuck::try_from_bytes(value)
    }
}

/// Array conversion: &[u8;N] --> &Page<WORDS>
impl<'a,const N: usize, const WORDS:usize> TryFrom<&'a [u8;N]> for &'a Page<WORDS> {
    type Error = bytemuck::PodCastError;
    fn try_from(value: &'a [u8; N]) -> Result<Self, Self::Error> {
        bytemuck::try_from_bytes(value)
    }
}

/// Vector conversion: &Vec<u8> --> &Page<WORDS>
impl<'a,const WORDS:usize> TryFrom<&'a Vec<u8>> for &'a Page<WORDS> {
    type Error = bytemuck::PodCastError;
    fn try_from(value: &'a Vec<u8>) -> Result<Self, Self::Error> {
        bytemuck::try_from_bytes(value)
    }
}

impl<const WORDS:usize> fmt::Display for Page<WORDS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Seed:    0x{:016X}\n", self.seed)?;
        write!(f, "FileID:  0x{:016X}\n", self.file)?;
        write!(f, "PageID:  0x{:016X}\n", self.page)?;
        write!(f, "MutCnt:  0x{:016X}\n", self.mutations)?;
        write!(f, "Data:\n")?;
        for i in 0..WORDS {
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
unsafe impl<const WORDS:usize> Pod for Page<WORDS> {}
unsafe impl<const WORDS:usize> Zeroable for Page<WORDS> {
    fn zeroed() -> Self {
        Page::default()
    }
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
            const WORDS: usize = 1;
            const PAGE_SIZE: usize = METADATA_SIZE + WORDS * 8;

            // &Page<WORDS> --> &[u8]
            let page: Page<WORDS> = Page::new(S, F, P);
            let bytes: &[u8] = bytemuck::bytes_of(&page);
            assert!(&flat_tv == bytes);

            // Box<Page<WORDS>> --> &[u8]
            let page: Page<WORDS> = Page::new(S, F, P);
            let page_box: Box<Page<WORDS>> = Box::new(page);
            let bytes: &[u8] = bytemuck::bytes_of(page_box.as_ref());
            assert!(&flat_tv == bytes.as_ref());

            // test that the pointers for Box<Page<WORDS>> --> &[u8] are the same.
            let page_box_ptr: *const Page<WORDS> = &*page_box;
            let bytes_ptr: *const [u8] = &*bytes;
            assert!(format!("{page_box_ptr:?}") == format!("{bytes_ptr:?}"), "Pointers are not equal: {page_box_ptr:?} != {bytes_ptr:?}");
           

            // Box<Page<WORDS>> --> &[u8]
            let bytes: &[u8] = page_box.as_bytes();
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
            const WORDS: usize = 1;
            const PAGE_SIZE: usize = METADATA_SIZE + WORDS * 8;
            const PAGES: usize = 2;

            // From Array -----------------------------------
            // &[Page<Words>; 2] --> &[u8]
            let page1: Page<WORDS> = Page::new(S, F, P);
            let page2: Page<WORDS> = Page::new(S, F, P + 1);
            let pages: [Page<WORDS>; 2] = [page1, page2];

            let bytes: &[u8] = bytemuck::bytes_of(&pages);

            assert!(flat_tv == bytes);

            // [Page<WORDS>; PAGES] --> Vec<u8>
            let bytes: Vec<u8> = pages.iter().flat_map(|p| {
                let bytes: &[u8] = p.try_into().expect("Unable to convert");
                bytes.to_vec()
            }).collect();
            assert!(flat_tv == bytes);

            // From Box ------------------------------------------
            // Box<[Page<WORDS>]> --> &[u8]
            let pages: Box<[Page<WORDS>]> = Box::new([
                Page::new(S, F, P), Page::new(S, F, P+1)
            ]);
            let bytes: &[u8] = bytemuck::bytes_of(TryInto::<&[Page<WORDS>;PAGES]>::try_into(pages.as_ref()).expect("Unable to convert"));
            assert!(flat_tv == bytes);
            
            // From Vector ---------------------------------------------------
            // Vec<Page<WORDS>> --> Vec<u8>
            let mut pages: Vec<Page<WORDS>> = Vec::with_capacity(PAGES);
            pages.push(Page::new(S, F, P));
            pages.push(Page::new(S, F, P+1));
           
            let bytes: Vec<u8> = pages.into_boxed_slice().iter().flat_map(|p| {
                let pbytes: &[u8] = p.try_into().expect("Unable to convert");
                pbytes.to_vec()
            }).collect();
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
            const WORDS: usize = 1;
            let page_tv: Page<WORDS> = Page::new(S, F, P);

            // Test different single page transformations
            let pages: [&Page<WORDS>; 4] = [
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

            const WORDS: usize = 1;
            const PAGES: usize = 2;
            let pages_tv: [Page<WORDS>; PAGES] = [
                Page::new(S, F, P),
                Page::new(S, F, P+1)
            ];

            // From Vector ------------------------------
            // Vec<u8> --> Vec<Page<WORDS>>
            let pages: Vec<Page<WORDS>> = bytemuck::try_from_bytes::<[Page<WORDS>; PAGES]>(&flat_tv)
                                                        .expect("Could not convert bytes to Page!")
                                                        .to_vec();
            assert!(*pages == pages_tv);

            // Vec<u8> --> &[Page<WORDS>]
            let pages: &[Page<WORDS>] = bytemuck::try_from_bytes::<[Page<WORDS>; PAGES]>(&flat_tv)
                                                        .expect("Could not convert bytes to Page!");
            assert!(*pages == pages_tv);
            
            // Vec<u8> --> &[Page<WORDS>; PAGES]
            let pages: &[Page<WORDS>; PAGES] = bytemuck::try_from_bytes::<[Page<WORDS>; PAGES]>(&flat_tv)
                                                        .expect("Could not convert bytes to Page!");
            assert!(*pages == pages_tv);


            // From Array ---------------------------------
            let flat_tv: &[u8] = flat_tv.as_slice();

            // &[u8] --> &[Page<WORDS>; PAGES]
            let pages: &[Page<WORDS>;PAGES] = bytemuck::try_from_bytes(flat_tv)
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
            const WORDS: usize = (PAGE_SIZE - (METADATA_SIZE)) / 8;

            let mut rng: ThreadRng = rand::thread_rng();

            let seed: u64 = rng.gen();
            let file: u64 = rng.gen();

            let pages: [Page<WORDS>; PAGE_COUNT] = array_init(|i: usize| Page::new(seed, file, i as u64));
            let pages_bytes: &[u8; PAGE_COUNT * PAGE_SIZE] = to_byte_slice(&pages);

            let pages: &[Page<WORDS>; PAGE_COUNT] = from_byte_slice(pages_bytes).expect("Could not transmute page!");

            for (p, page) in pages.iter().enumerate() {
                assert!(page.validate_page_with(seed, file, p as u64, 0)); 
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
            const WORDS: usize = (PAGE_SIZE - (METADATA_SIZE)) / 8;

            let mut rng: ThreadRng = rand::thread_rng();

            let seed: u64 = rng.gen();
            let file: u64 = rng.gen();

            let pages: Vec<Page<WORDS>> = (0..PAGE_COUNT).map(|i|{
                    Page::new(seed, file, i as u64)
                }).collect();
            
            
            for (p, page) in pages.iter().enumerate() {
                assert!(page.validate_page_with(seed, file, p as u64, 0)); 
            }
        }
        #[test]
        fn vec_writes_and_reads() {
            /* This test fails. Rather than trying to manually transmute between types, I should
             * use invest in learning ByteMuck. Here is an example project that goes from
             * u8 -> T, https://github.com/MolotovCherry/virtual-display-rs/blob/master/virtual-display-driver/src/edid.rs
             * If I go this route then I should embed bytemuck in From/To traits for following
             * types:
             *  - Vec<Page<WORDS>> and Vec<u8>
             *  - &[Page<Words] and &[u8]
             *
             */
            use super::super::{
                Page,
                METADATA_SIZE,
            };
            use rand::prelude::*;
            //use array_init::array_init;
            use std::{
                fs::File,
                io::Write,
            };

            const PAGE_SIZE: usize = 512;
            const PAGE_COUNT: usize = 64;
            const WORDS: usize = (PAGE_SIZE - (METADATA_SIZE)) / 8;
            let tmpfile_name: String = String::from("test.page.serde");
            let mut tmpfile: File = File::create(tmpfile_name.clone()).expect("Was not able to create temporary file!");

            let mut rng: ThreadRng = rand::thread_rng();

            let seed: u64 = rng.gen();
            let file: u64 = rng.gen();

            let pages: Vec<Page<WORDS>> = (0..PAGE_COUNT).map(|i|{
                    Page::new(seed, file, i as u64)
                }).collect();

            let write_buffer: Vec<u8> = pages.into_iter()
                                             .map(|p|{ p.as_bytes().to_vec() })
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

            let pages_w: &[Page<WORDS>; PAGE_COUNT] = bytemuck::try_from_bytes::<[Page<WORDS>; PAGE_COUNT]>(&read_buffer.as_slice())
                                                        .expect("Could not convert bytes to Page");

            for (p, page) in pages_w.iter().enumerate() {
                if !page.validate_page_with(seed, file, p as u64, 0) {
                    std::fs::remove_file(tmpfile_name.clone()).expect("Unable to remove temporary testing file");
                    assert!(false, "Failed to valid page {} of {}", p, PAGE_COUNT);
                }
            }
            std::fs::remove_file(tmpfile_name.clone()).expect("Unable to remove temporary testing file");
        }
    }
}
