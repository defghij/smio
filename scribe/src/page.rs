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

    pub fn to_byte_slice<'a>(self) -> &'a [u8] {
        let ptr =  &self as *const Page<N> as *const u8;
        unsafe {
            std::slice::from_raw_parts(ptr, std::mem::size_of::<Page<N>>())
        }
    }
    pub fn from_byte_slice<'a>(slice: &[u8]) -> Option<&Page<N>> {
        if slice.len() != std::mem::size_of::<Page<N>>() {
            return None;
        }
        let ptr = slice.as_ptr() as *const Page<N>;
        Some(unsafe {&*ptr })
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

#[test]
fn serialize_page() {
    let flat_tv  = [
        0xC0, 0xC1, 0xC2, 0xC3, // preseed
        0xB0, 0xB1, 0xB2, 0xB3, // file
        0xA0, 0xA1, 0xA2, 0xA3, // page
        0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
        0xAC, 0x7E, 0x13,  0xB5, // data segment...
        0xC5, 0x33, 0x89, 0x4E  //   data cont'd
    ];
    let prsd: u32 = 0xC3C2C1C0;
    let f: u32 = 0xB3B2B1B0;
    let p: u64 = 0xA7A6A5A4A3A2A1A0;
    const DATA_SIZE: usize = 1;

    let page: Page<DATA_SIZE> = Page::new(prsd, f, p);
    let flat: &[u8] = page.to_byte_slice();

    // Test slices
    assert!(flat_tv[0..4]  == flat[0..4]);
    assert!(flat_tv[4..8]  == flat[4..8]);
    assert!(flat_tv[8..16] == flat[8..16]);
    assert!(flat_tv[16..]  == flat[16..]);

}

#[test]
fn deserialize_page() {
    let flat_tv  = [
        0xC0, 0xC1, 0xC2, 0xC3, // preseed
        0xB0, 0xB1, 0xB2, 0xB3, // file
        0xA0, 0xA1, 0xA2, 0xA3, // page
        0xA4, 0xA5, 0xA6, 0xA7,  //  page cont'd
        0xAC, 0x7E, 0x13,  0xB5, // data segment...
        0xC5, 0x33, 0x89, 0x4E  //   data cont'd
    ];

    const DATA_SIZE: usize = 1;
    let page: &Page<DATA_SIZE> = Page::<DATA_SIZE>::from_byte_slice(&flat_tv).expect("Could not deserialize!");

    assert!(page.preseed == 0xC3C2C1C0);
    assert!(page.file    == 0xB3B2B1B0);
    assert!(page.page    == 0xA7A6A5A4A3A2A1A0);
    assert!(page.data[0] == 0x4E8933C5B5137EAC);
}
