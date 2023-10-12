pub mod page {

    use rand_xoshiro::rand_core::{RngCore, SeedableRng};
    use rand_xoshiro::Xoroshiro128PlusPlus;
    use bitcode::{Encode, Decode};
    use std::fmt;

    #[repr(C)]
    #[derive(Encode, Decode, Debug, Copy, Clone)]
    pub struct Page<const N: usize> {
        file: u32,
        page: u64,
        preseed: u32,
        data: [u64; N]
    } impl<const N: usize> Page<N> {
        #[allow(dead_code)]
        pub fn new(file: u32, page: u64, preseed: u32) -> Page<N> {
            let data: [u64; N] = Page::generate_data(Page::<N>::assemble_seed(file, page, preseed));
            Page::<N> {
                file,
                page,
                preseed,
                data
            }
        }
        
        #[allow(dead_code)]
        pub fn mutate_page(&mut self, new_preseed: u32) {
            self.preseed = new_preseed;
            let data: [u64; N] = Page::generate_data(Page::<1>::assemble_seed(self.file, self.page, self.preseed));
            self.data = data;
        }

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
        let f: u32 = u32::MAX - 1;
        let p: u64 = u64::MAX;
        let prsd: u32 = 0xdead;
        const DATA_SIZE: usize = 1;

        let page: Page<DATA_SIZE> = Page::new(f, p, prsd);
        let flat: Vec<u8> = bitcode::encode(&page).unwrap();
        
        assert!([u8::MAX - 1, u8::MAX, u8::MAX, u8::MAX] == flat[0..4]);
        assert!([u8::MAX; 8] == flat[4..12]);
        assert!([0xad, 0xde, 0,0] == flat[12..16]);

        let seed: u64 = Page::<DATA_SIZE>::assemble_seed(f, p, prsd);
        let mut rng = Xoroshiro128PlusPlus::seed_from_u64(seed);
       
        let data: u64 = rng.next_u64();
        let data: [u8; 8] = data.to_le_bytes();
        assert!(data == flat[16..]);
    }

    #[test]
    fn deserialize_page() {
        let flat  = [
            0xfe, 0xff, 0xff, 0xff, // u32::MAX - 1
            0xff, 0xff, 0xff, 0xff, // u64::MAX
            0xff, 0xff, 0xff, 0xff, //   cont'd
            0xad, 0xde, 0x0,  0x0,  // 0xdead
            0x16, 0x66, 0x4,  0x67, // data segment...
            0x5d, 0x93, 0xbf, 0xf2  //   cont'd
        ];
        const DATA_SIZE: usize = 1;

        let page: Page<DATA_SIZE> = bitcode::decode(&flat).unwrap();
        let file_id = page.file;
        let page_id = page.page;
        let preseed = page.preseed;
        let data: [u64; DATA_SIZE] = page.data;

        let seed: u64 = Page::<DATA_SIZE>::assemble_seed(file_id, page_id, preseed);
        let mut rng = Xoroshiro128PlusPlus::seed_from_u64(seed);
        assert!(data[0] == rng.next_u64());
    }
}
