//mod scribe;
pub mod page;
pub mod bookcase;


// Bookcase structure
pub const PAGE_SIZE: usize       = 4096 /*bytes*/;
pub const PAGE_COUNT: usize      = 512;
pub const PAGES_PER_WRITE: usize = 256;

// Page Structure
pub const DATA_SIZE: usize     = PAGE_SIZE - page::METADATA_SIZE /*bytes*/;
pub const WORDS: usize    = DATA_SIZE / 8;  /*u64s*/

pub type PageBytes = [u8; PAGE_SIZE];

pub mod memory_ops {

    pub fn to_byte_slice<'a, S, D>(obj: &S) -> &'a D
    where
        D: Sized,
        S: Sized 
    {
        unsafe {
            std::mem::transmute(obj)
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

pub mod secretary {

    pub mod scheduler { 
        use std::sync::{
            Arc, atomic::AtomicU64
        };

        fn to_1d(x: u64, y: u64, z: u64, w: u64, h: u64) -> u64 {
            z * (w * h) + y * w + x
        }

        pub struct WorkUnit( (u32, u32, u64) );
        impl WorkUnit {
            #[inline(always)]
            pub fn shelf(self) -> u32 {
                self.0.0
            }
            #[inline(always)]
            pub fn book(self) -> u32 {
                self.0.1
            }
            #[inline(always)]
            pub fn page(self) -> u64 {
                self.0.2
            }
        }

        /// A type to manage the conversion and assignment of work.
        /// The work this type manages can be thought of as a DxFxP matrix
        /// where:
        ///     D = total number of directories,
        ///     F = total number of files,
        ///     P = total number of pages
        /// Thus, the total work this iterator may yield is D x F x P. It is
        /// concievable that work may be distributed among threads or processes
        /// such that the work for any individual WorkRangeIterator < D x F x P. 
        /// In that case, the total work is:
        ///             (D_{2} - D_{1}) x (F_{2} - F_{1}) x (P_{2} - P_{1})
        /// Thus, it takes four arguments:
        ///     start: WorkUnit
        ///     end: Workunit
        ///     step: u64
        pub struct WorkRangeIterator<const D: u64, const F: u64, const P: u64>(Arc<AtomicU64>);
        impl<const D: u64, const F: u64, const P: u64> Iterator for WorkRangeIterator<D,F,P> {
            type Item = (WorkUnit, WorkUnit);

            fn next(&mut self) -> Option<Self::Item> {
                unimplemented!();
            }
        } impl<const D: u64, const F: u64, const P: u64> From<u64> for WorkRangeIterator<D,F,P> {
            fn from(item: u64) -> WorkRangeIterator<D,F,P> {
                WorkRangeIterator(Arc::new(AtomicU64::new(item)))
            }
        }

        mod work_distribution { 
        }
    }
}

