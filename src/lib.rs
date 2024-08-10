pub mod page;
pub mod chapter;
pub mod bookcase;
pub mod queue;

pub const PAGE_BYTES: usize         = 4096 /*bytes*/;
pub const METADATA_BYTES: usize    = page::Page::<0>::METADATA_BYTES;
pub const DATA_BYTES: usize        = PAGE_BYTES - METADATA_BYTES;
pub const DATA_WORDS: usize        = DATA_BYTES / std::mem::size_of::<u64>();
pub const PAGE_COUNT: usize        = 512;
pub const PAGES_PER_CHAPTER: usize = 256;

pub type PageBytes = [u8; PAGE_BYTES];

