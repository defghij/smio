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
                              
pub const METADATA_SIZE: usize = 16 /*bytes*/;
use super::PAGE_SIZE;

/* TODO: 
 *  - Add a mutation count parameter that adds to the base preseed to yield new data
 *  - WORDS should be words for the total structure. Not just the data segment
 */ 
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Page<const WORDS: usize> {
    preseed: u32,
    file: u32,
    page: u64,
    data: [u64; WORDS]
} impl<const WORDS: usize> Page<WORDS> {
    #[allow(dead_code)]
    pub fn new(preseed: u32, file: u32, page: u64) -> Page<WORDS> {
        let data: [u64; WORDS] = Page::generate_data(Page::<WORDS>::assemble_seed(file, page, preseed));
        Page::<WORDS> {
            file,
            preseed,
            page,
            data
        }
    }

    pub fn default() -> Page<WORDS> {
        Page::<WORDS> {
            file: 0,
            preseed: 0,
            page: 0,
            data: [0u64; WORDS]
        }
    }

    
    ////////////////////////////////////////////////////
    //// Data Functions
    #[allow(dead_code)]
    fn assemble_seed(file: u32, page: u64, preseed: u32) -> u64 {
        let seed_page: u64 = page << 32;   
        let seed_file: u64 = !(file as u64) << 46;
        let seed_upper: u64 = seed_file | seed_page;
        let seed_lower: u64 = preseed as u64;
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

    pub fn validate_page_with(self, preseed: u32, file: u32, page: u64) -> bool {
        let data: [u64; WORDS] = Page::generate_data(Page::<0>::assemble_seed(file, page, preseed));
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
    pub fn reinit(&mut self, preseed: u32, file: u32, page: u64) {
        self.preseed = preseed;
        self.file = file;
        self.page = page;
        self.data = Page::generate_data(Page::<WORDS>::assemble_seed(file, page, preseed));
    }

    #[allow(dead_code)]
    pub fn mutate_seed(&mut self, preseed: u32) {
        self.preseed = preseed;
        let data: [u64; WORDS] = Page::generate_data(Page::<0>::assemble_seed(self.file, self.page, self.preseed));
        self.data = data;
    }

    #[allow(dead_code)]
    pub fn mutate_file(&mut self, file: u32) {
        self.file = file;
        let data: [u64; WORDS] = Page::generate_data(Page::<1>::assemble_seed(self.file, self.page, self.preseed));
        self.data = data;
    }

    #[allow(dead_code)]
    pub fn mutate_page(&mut self, page: u64) {
        self.page = page;
        let data: [u64; WORDS] = Page::generate_data(Page::<1>::assemble_seed(self.file, self.page, self.preseed));
        self.data = data;
    }
}

impl<const WORDS:usize> PartialEq for Page<WORDS> {
    fn eq(&self, other: &Self) -> bool {
        if self.preseed != other.preseed { return false; }
        if self.file != other.file { return false; }
        if self.page != other.page { return false; }
        if self.data != other.data { return false; }
        true
    }
}

impl<const WORDS:usize> fmt::Display for Page<WORDS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PreSeed: 0x{:08X}\n", self.preseed)?;
        write!(f, "FileID:  0x{:08X}\n", self.file)?;
        write!(f, "PageID:  0x{:016X}\n", self.page)?;
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
mod transmutation {
    pub const S: u32 = 0xC3C2C1C0;
    pub const F: u32 = 0xB3B2B1B0;
    pub const P: u64 = 0xA7A6A5A4A3A2A1A0;
    pub const D1: u64 = 0x4E8933C5B5137EAC;
    pub const D2: u64 = 0x45639083B573314C;

    mod to_u8 {
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
            let flat_tv: [u8; 24]  = [
                0xC0, 0xC1, 0xC2, 0xC3,  // preseed
                0xB0, 0xB1, 0xB2, 0xB3,  // file
                0xA0, 0xA1, 0xA2, 0xA3,  // page
                0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
                0xAC, 0x7E, 0x13, 0xB5,  // data segment...
                0xC5, 0x33, 0x89, 0x4E   //  data cont'd
            ];
            const WORDS: usize = 1;
            const PAGE_SIZE: usize = METADATA_SIZE + WORDS * 8;

            let page: Page<WORDS> = Page::new(S, F, P);
            //let flat: &[u8; PAGE_SIZE] = to_byte_slice(&page);
            let flat: &[u8] = bytemuck::bytes_of(&page);

            // Test slices
            assert!(&flat_tv  == flat);
        }

        fn two_pages_array_and_vector() {
            use super::{
                S, F, P,
                super::{
                    Page,
                    METADATA_SIZE,
                }
            };
            use bytemuck;
            let flat_tv  = vec![
                0xC0, 0xC1, 0xC2, 0xC3,  // preseed
                0xB0, 0xB1, 0xB2, 0xB3,  // file
                0xA0, 0xA1, 0xA2, 0xA3,  // page
                0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
                0xAC, 0x7E, 0x13, 0xB5,  // data segment...
                0xC5, 0x33, 0x89, 0x4E,  //  data cont'd
                0xB0, 0xB1, 0xB2, 0xB3,  // file
                0xC0, 0xC1, 0xC2, 0xC3,  // preseed
                0xA0, 0xA1, 0xA2, 0xA3,  // page
                0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
                0x4C, 0x31, 0x73, 0xB5,  // data segment...
                0x83, 0x90, 0x63, 0x45   //  data cont'd
            ];
            const WORDS: usize = 1;
            const PAGE_SIZE: usize = METADATA_SIZE + WORDS * 8;
            const PAGES: usize = 2;

            // Array -----------------------------------
            let page1: Page<WORDS> = Page::new(S, F, P);
            let page2: Page<WORDS> = Page::new(F, S, P);
            let pages: [Page<WORDS>; 2] = [page1, page2];

            let pages_bytes: &[u8] = bytemuck::bytes_of(&pages);

            assert!(&flat_tv == pages_bytes);
        
            // Vector ---------------------------------------------------
            let mut pages: Vec<Page<WORDS>> = Vec::with_capacity(PAGES);
            pages.push(Page::new(S, F, P));
            pages.push(Page::new(F, S, P));
            
            let pages: Box<[Page<WORDS>]> = pages.into_boxed_slice();
            assert!(pages.len() == 2);

            let pages_bytes: Vec<u8> = pages.iter().map(|p| {
                bytemuck::bytes_of(p).to_vec()
            }).flatten().collect();

            assert!(flat_tv == pages_bytes);
        }

    }

    mod from_u8 {
        #[test]
        fn single_page() {
            use super::{
                S, F, P, D1,
                super::{
                    Page,
                    super::memory_ops::from_byte_slice
                }
            };
            use bytemuck;
            let flat_tv: [u8; 24]  = [
                0xC0, 0xC1, 0xC2, 0xC3,  // preseed
                0xB0, 0xB1, 0xB2, 0xB3,  // file
                0xA0, 0xA1, 0xA2, 0xA3,  // page
                0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
                0xAC, 0x7E, 0x13, 0xB5, // data segment...
                0xC5, 0x33, 0x89, 0x4E   //  data cont'd
            ];
            const WORDS: usize = 1;

            let page: &Page<WORDS> = from_byte_slice(&flat_tv).expect("Could not deserialize!");
            let page: &Page<WORDS> = bytemuck::try_from_bytes(&flat_tv).expect("Could not convert bytes to Page!");

            assert!(page.preseed == S,  "{:X} != {:X}", page.preseed, S);
            assert!(page.file    == F,  "{:X} != {:X}", page.file, F);
            assert!(page.page    == P,  "{:X} != {:X}", page.page, P );
            assert!(page.data[0] == D1, "{:X} != {:X}", page.data[0], D1);
        }

        #[test]
        fn two_pages_array_and_vector() {
            use super::{
                S, F, P, D1, D2,
                super::{
                    Page,
                }
            };
            use bytemuck;
            let flat_tv  = [
                0xC0, 0xC1, 0xC2, 0xC3,  // preseed
                0xB0, 0xB1, 0xB2, 0xB3,  // file
                0xA0, 0xA1, 0xA2, 0xA3,  // page
                0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
                0xAC, 0x7E, 0x13, 0xB5, // data segment...
                0xC5, 0x33, 0x89, 0x4E,  //  data cont'd
                0xB0, 0xB1, 0xB2, 0xB3,  // file
                0xC0, 0xC1, 0xC2, 0xC3,  // preseed
                0xA0, 0xA1, 0xA2, 0xA3,  // page
                0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
                0x4C, 0x31, 0x73, 0xB5,  // data segment...
                0x83, 0x90, 0x63, 0x45   //  data cont'd
            ];
            const WORDS: usize = 1;
            const PAGES: usize = 2;

            // Array ---------------------------------
            let pages_a: &[Page<WORDS>;PAGES] = bytemuck::try_from_bytes(&flat_tv).expect("Could not convert bytes to Page!");

            assert!(pages_a[0].preseed == S,  "{:X} != {:X}", pages_a[0].preseed, S);
            assert!(pages_a[0].file    == F,  "{:X} != {:X}", pages_a[0].file, F);
            assert!(pages_a[0].page    == P,  "{:X} != {:X}", pages_a[0].page, P);
            assert!(pages_a[0].data[0] == D1, "{:X} != {:X}", pages_a[0].data[0], D1);
            assert!(pages_a[1].preseed == F,  "{:X} != {:X}", pages_a[1].preseed, F);
            assert!(pages_a[1].file    == S,  "{:X} != {:X}", pages_a[1].file, S);
            assert!(pages_a[1].page    == P,  "{:X} != {:X}", pages_a[1].page, P);
            assert!(pages_a[1].data[0] == D2, "{:X} != {:X}", pages_a[1].data[0], D2);

            // Vector ------------------------------
            let pages_b: Vec<Page<WORDS>> = bytemuck::try_from_bytes::<[Page<WORDS>; PAGES]>(&flat_tv)
                                                        .expect("Could not convert bytes to Page!")
                                                        .to_vec();

            assert!(pages_b[0].preseed == S,  "{:X} != {:X}", pages_b[0].preseed, S);
            assert!(pages_b[0].file    == F,  "{:X} != {:X}", pages_b[0].file, F);
            assert!(pages_b[0].page    == P,  "{:X} != {:X}", pages_b[0].page, P);
            assert!(pages_b[0].data[0] == D1, "{:X} != {:X}", pages_b[0].data[0], D1);
            assert!(pages_b[1].preseed == F,  "{:X} != {:X}", pages_b[1].preseed, F);
            assert!(pages_b[1].file    == S,  "{:X} != {:X}", pages_b[1].file, S);
            assert!(pages_b[1].page    == P,  "{:X} != {:X}", pages_b[1].page, P);
            assert!(pages_b[1].data[0] == D2, "{:X} != {:X}", pages_b[1].data[0], D2);
        }
    }


    mod to_and_from {
        #[test]
        fn random_page_bytes_slice() {
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

            let preseed: u32 = rng.gen();
            let file: u32 = rng.gen();

            let pages: [Page<WORDS>; PAGE_COUNT] = array_init(|i: usize| Page::new(preseed, file, i as u64));
            let pages_bytes: &[u8; PAGE_COUNT * PAGE_SIZE] = to_byte_slice(&pages);

            let pages: &[Page<WORDS>; PAGE_COUNT] = from_byte_slice(pages_bytes).expect("Could not transmute page!");

            for (p, page) in pages.iter().enumerate() {
                assert!(page.validate_page_with(preseed, file, p as u64)); 
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

            let preseed: u32 = rng.gen();
            let file: u32 = rng.gen();

            let pages: Vec<Page<WORDS>> = (0..PAGE_COUNT).map(|i|{
                    Page::new(preseed,file,i as u64)
                }).collect();
            
            
            for (p, page) in pages.iter().enumerate() {
                assert!(page.validate_page_with(preseed, file, p as u64)); 
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
                super::memory_ops::{
                    from_byte_slice
                }
            };
            use rand::prelude::*;
            //use array_init::array_init;
            use std::{
                fs::File,
                io::{
                    Write,
                    Read
                }
            };

            const PAGE_SIZE: usize = 512;
            const PAGE_COUNT: usize = 64;
            const WORDS: usize = (PAGE_SIZE - (METADATA_SIZE)) / 8;
            let tmpfile_name: String = String::from("test.page.serde");
            let mut tmpfile: File = File::create(tmpfile_name.clone()).expect("Was not able to create temporary file!");

            let mut rng: ThreadRng = rand::thread_rng();

            let preseed: u32 = rng.gen();
            let file: u32 = rng.gen();

            let pages: Vec<Page<WORDS>> = (0..PAGE_COUNT).map(|i|{
                    Page::new(preseed,file,i as u64)
                }).collect();

            let write_buffer: Vec<u8> = pages.into_iter()
                                             .map(|p|{ p.as_bytes().to_vec() })
                                             .flatten()
                                             .collect();

            // Transition from bits in address-space to bits in file-space
            tmpfile.write_all(write_buffer.as_slice()).unwrap();
            tmpfile.flush().expect("Could not flush temporary file");

            //drop(tmpfile); // Let OS/Rust reap this file descriptor.

            // Transition from bits in file-space to bits in address-space
            let mut tmpfile: File = File::open(tmpfile_name.clone()).expect("Was not able to open temporary file!");
            let mut read_buffer: Vec<u8> = Vec::with_capacity(PAGE_COUNT * PAGE_SIZE);
            let _ = tmpfile.read_exact(read_buffer.as_mut_slice());
            assert!(read_buffer.len() == PAGE_SIZE * PAGE_COUNT);

            //let pages_w: &[Page<WORDS>; PAGE_COUNT] = from_byte_slice(&read_buffer).expect("Could not transmute page!");
            let pages_w: Vec<Page<WORDS>> = bytemuck::try_from_bytes::<[Page<WORDS>; PAGE_COUNT]>(&read_buffer.as_slice())
                                                        .expect("Could not convert bytes to Page!")
                                                        .to_vec();

            for (p, page) in pages_w.iter().enumerate() {
                assert!(page.validate_page_with(preseed, file, p as u64));
            }
            std::fs::remove_file(tmpfile_name.clone()).expect("Unable to remove temporary testing file");
        }

        #[test]
        fn file_system() {
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
            use std::{
                fs::File,
                io::{
                    Write,
                    Read
                }
            };

            const PAGE_SIZE: usize = 512;
            const PAGE_COUNT: usize = 64;
            const WORDS: usize = (PAGE_SIZE - (METADATA_SIZE)) / 8;
            let tmpfile_name: String = String::from("test.page.serde");
            let mut tmpfile: File = File::create(tmpfile_name.clone()).expect("Was not able to create temporary file!");

            let mut rng: ThreadRng = rand::thread_rng();

            let preseed: u32 = rng.gen();
            let file: u32 = rng.gen();

            let pages_r: [Page<WORDS>; PAGE_COUNT] = array_init(|i: usize| Page::new(preseed, file, i as u64));
            let write_buffer: &[u8; PAGE_COUNT * PAGE_SIZE] = to_byte_slice(&pages_r);


            // Transition from bits in address-space to bits in file-space
            tmpfile.write_all(write_buffer).unwrap();
            tmpfile.flush().expect("Could not flush temporary file");

            drop(tmpfile); // Let OS/Rust reap this file descriptor.

            // Transition from bits in file-space to bits in address-space
            let mut tmpfile: File = File::open(tmpfile_name.clone()).expect("Was not able to open temporary file!");
            let mut read_buffer: [u8; PAGE_COUNT * PAGE_SIZE] = [0; PAGE_COUNT * PAGE_SIZE]; 
            let _ = tmpfile.read_exact(&mut read_buffer);


            let pages_w: &[Page<WORDS>; PAGE_COUNT] = from_byte_slice(&read_buffer).expect("Could not transmute page!");

            for (p, page) in pages_w.iter().enumerate() {
                if !page.validate_page_with(preseed, file, p as u64) {
                    //std::fs::remove_file(tmpfile_name.clone()); Leave file for troubleshooting test
                    // failure
                    assert!(false);
                } 
            }
            std::fs::remove_file(tmpfile_name.clone()).expect("Unable to remove temporary testing file");
        }
    }

}
