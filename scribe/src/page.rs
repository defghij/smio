use rand_xoshiro::rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoroshiro128PlusPlus;
use bitcode::{Encode, Decode};
use std::fmt;


//TODO
//-----------------------
// 1. Need to be able to write pages an array in memory? Maybe the stack is sufficient?

#[repr(C)]
#[derive(Encode, Decode, Debug, Copy, Clone)]
pub struct Page<const N: usize> {
    preseed: u32,
    file: u32,
    page: u64,
    data: [u64; N]
} impl<const N: usize> Page<N> {
    #[allow(dead_code)]
    pub fn new(preseed: u32, file: u32, page: u64) -> Page<N> {
        let data: [u64; N] = Page::generate_data(Page::<N>::assemble_seed(file, page, preseed));
        Page::<N> {
            file,
            preseed,
            page,
            data
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
    
    fn generate_data(seed: u64) -> [u64; N] {
        let mut rng = Xoroshiro128PlusPlus::seed_from_u64(seed);
        let data: [u64; N] = {
            let mut temp = [0; N];
            for elem in temp.iter_mut() {
                *elem = rng.next_u64();
            }
            temp
        };
        data
    }

    ////////////////////////////////////////////////////
    //// Mutatate Functions
    #[allow(dead_code)]
    pub fn mutate_seed(&mut self, preseed: u32) {
        self.preseed = preseed;
        let data: [u64; N] = Page::generate_data(Page::<0>::assemble_seed(self.file, self.page, self.preseed));
        self.data = data;
    }
    #[allow(dead_code)]
    pub fn mutate_file(&mut self, file: u32) {
        self.file = file;
        let data: [u64; N] = Page::generate_data(Page::<1>::assemble_seed(self.file, self.page, self.preseed));
        self.data = data;
    }
    #[allow(dead_code)]
    pub fn mutate_page(&mut self, page: u64) {
        self.page = page;
        let data: [u64; N] = Page::generate_data(Page::<1>::assemble_seed(self.file, self.page, self.preseed));
        self.data = data;
    }
}
impl<const N:usize> fmt::Display for Page<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileID:  0x{:x}\n", self.file)?;
        write!(f, "PageID:  0x{:x}\n", self.page)?;
        write!(f, "PreSeed: 0x{:x}\n", self.preseed)?;
        write!(f, "Data:\n")?;
        for i in 0..N {
            let bytes = self.data[i].to_be_bytes();
            write!(f, "{:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x}\n",
                   bytes[0], bytes[1], bytes[2], bytes[3],
                   bytes[4], bytes[5], bytes[6], bytes[7])?;
        }
        Ok(())
    }
}

pub mod memory_ops {

    pub fn to_byte_slice<'a, T>(obj: &T) -> &'a [u8] {
        let ptr =  obj as *const T as *const u8;
        unsafe {
            std::slice::from_raw_parts(ptr, std::mem::size_of::<T>())
        }
    }
    pub fn from_byte_slice<'a, T>(slice: &[u8]) -> Option<&T> {
        if slice.len() != std::mem::size_of::<T>() {
            return None;
        }
        let ptr = slice.as_ptr() as *const T;
        Some(unsafe {&*ptr })
    }
}

pub mod page_testing{
    pub const S: u32 = 0xC3C2C1C0;
    pub const F: u32 = 0xB3B2B1B0;
    pub const P: u64 = 0xA7A6A5A4A3A2A1A0;
    pub const D1: u64 = 0x4E8933C5B5137EAC;
    pub const D2: u64 = 0x45639083B573314C;

    #[test]
    fn serialize_single_page() {
        use super::Page;
        use super::memory_ops::to_byte_slice;
        let flat_tv: [u8; 24]  = [
            0xC0, 0xC1, 0xC2, 0xC3,  // preseed
            0xB0, 0xB1, 0xB2, 0xB3,  // file
            0xA0, 0xA1, 0xA2, 0xA3,  // page
            0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
            0xAC, 0x7E, 0x13, 0xB5, // data segment...
            0xC5, 0x33, 0x89, 0x4E   //  data cont'd
        ];
        const DATA_SIZE: usize = 1;

        let page: Page<DATA_SIZE> = Page::new(S, F, P);
        let flat: &[u8] = to_byte_slice(&page);

        // Test slices
        assert!(flat_tv  == flat);
    }

    #[test]
    fn serialize_mulit_page() {
        use super::Page;
        use super::memory_ops::to_byte_slice;
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
        const DATA_SIZE: usize = 1;

        let page1: Page<DATA_SIZE> = Page::new(S, F, P);
        let page2: Page<DATA_SIZE> = Page::new(F, S, P);
        let pages: [Page<DATA_SIZE>; 2] = [page1, page2];

        let pages_bytes = to_byte_slice(&pages);

        assert!(flat_tv  == pages_bytes);
    }

    #[test]
    fn deserialize_single_page() {
        use super::Page;
        use super::memory_ops::from_byte_slice;
        let flat_tv: [u8; 24]  = [
            0xC0, 0xC1, 0xC2, 0xC3,  // preseed
            0xB0, 0xB1, 0xB2, 0xB3,  // file
            0xA0, 0xA1, 0xA2, 0xA3,  // page
            0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
            0xAC, 0x7E, 0x13, 0xB5, // data segment...
            0xC5, 0x33, 0x89, 0x4E   //  data cont'd
        ];
        const DATA_SIZE: usize = 1;
        let page: &Page<DATA_SIZE> = from_byte_slice(&flat_tv).expect("Could not deserialize!");

        assert!(page.preseed == S,  "{:X} != {:X}", page.preseed, S);
        assert!(page.file    == F,  "{:X} != {:X}", page.file, F);
        assert!(page.page    == P,  "{:X} != {:X}", page.page, P );
        assert!(page.data[0] == D1, "{:X} != {:X}", page.data[0], D1);
    }
    #[test]
    fn deserialize_mulit_page() {
        use super::Page;
        use super::memory_ops::from_byte_slice;
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
        const DATA_SIZE: usize = 1;
        let pages: &[Page<DATA_SIZE>; 2] = from_byte_slice(&flat_tv).expect("Could not deserialize!");

        assert!(pages[0].preseed == S,  "{:X} != {:X}", pages[0].preseed, S);
        assert!(pages[0].file    == F,  "{:X} != {:X}", pages[0].file, F);
        assert!(pages[0].page    == P,  "{:X} != {:X}", pages[0].page, P);
        assert!(pages[0].data[0] == D1, "{:X} != {:X}", pages[0].data[0], D1);
        assert!(pages[1].preseed == F,  "{:X} != {:X}", pages[1].preseed, F);
        assert!(pages[1].file    == S,  "{:X} != {:X}", pages[1].file, S);
        assert!(pages[1].page    == P,  "{:X} != {:X}", pages[1].page, P);
        assert!(pages[1].data[0] == D2, "{:X} != {:X}", pages[1].data[0], D2);
    }
}
