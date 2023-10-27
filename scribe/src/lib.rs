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
            atomic::AtomicU64
        };

        fn to_1d(x: u64, y: u64, z: u64, w: u64, h: u64) -> u64 {
            z * (w * h) + y * w + x
        }

        macro_rules! gen_assert {
            ($t:ident, $c:expr) => {{
                struct Check<$t>($t);
                impl<$t> Check<$t> {
                    const CHECK: () = assert!($c);
                }
                let _ = Check::<$t>::CHECK;
            }}
        }

        #[inline(always)]
        fn get_grid_element<const X: u64, const S: u64>(i: u64) -> (u64, u64) {
                let x0: u64 = i.rem_euclid(X);
                let y0: u64 = i / X;
                (x0, y0)
        }

        pub struct WorkUnit( (u32, u64) );

        pub struct WorkRangeIterator<const X: u64, const Y: u64, const S: u64, const M: u64>(AtomicU64);
        impl<const X: u64, const Y: u64, const S: u64, const M: u64> Iterator for WorkRangeIterator<X,Y,S,M> {
            type Item = (WorkUnit, WorkUnit);
                
            fn next(&mut self) -> Option<Self::Item> {
                // X = PAGES,
                // Y = FILES,
                // S = STEP,
                // M = MAX
                // Three cases, since start <= stop:
                //      1. (start, stop) -> WorkUnit,WorkUnit
                //      2. (start, stop] -> WorkUnit,WorkUnit
                //      3. [start, stop] -> None
                let i: u64 = self.0.fetch_add(S, std::sync::atomic::Ordering::SeqCst);

                // Case 3: [start, stop] -> None
                if i >= (M) { 
                    return None;
                }

                let (x0,y0): (u64, u64) = get_grid_element::<X,S>(i);
                let start: WorkUnit = WorkUnit( (y0 as u32, x0 ) );

                let (x1,y1): (u64, u64) = get_grid_element::<X,S>( (i + S) - 1);

                let end: WorkUnit;

                if y1 < Y { 
                    end = WorkUnit( (y1 as u32, x1));
                } else { // end = EOQ
                    end = WorkUnit( ((Y-1) as u32, (X - 1)));
                }
                Some( (start, end) )
            }
        } impl<const X: u64, const Y: u64, const S: u64, const M: u64> From<u64> for WorkRangeIterator<X,Y,S,M> {
            fn from(item: u64) -> WorkRangeIterator<X,Y,S,M> {
                WorkRangeIterator(AtomicU64::new(item))
            }
        }

        mod tests {

            #[test]
            fn grid2x1_step1_work2_st() {
                use super::{WorkUnit, WorkRangeIterator};
                let queue: WorkRangeIterator<2,1,1,2> = 0.into();

                let mut counter: u64 = 0;
                for work in queue {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u32 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u32 = end.0.0;
                    
                    match counter {
                        0 => {
                                                                // +-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |  W  |     |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(x1 == 0, "{} ? {}", x1, 0); 
                            assert!(y1 == 0, "{} ? {}", y1, 0);  
                        },
                        1 => {
                                                                // +-----+-----+
                            assert!(x0 == 1, "{} ? {}", x0, 1); // |     |  W  |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(x1 == 1, "{} ? {}", x1, 1); 
                            assert!(y1 == 0, "{} ? {}", y1, 0);  
                        },
                        _ => unreachable!("Work iterator iterated to far!"),
                    }
                    counter += 1;
                }
                assert!(counter == 2, "{} ? {}", counter, 2);
            }
           
            #[test]
            fn grid2x1_step2_work2_st() {
                use super::{WorkUnit, WorkRangeIterator};
                let queue: WorkRangeIterator<2,1,2,2> = 0.into();

                let mut counter: u64 = 0;
                for work in queue {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u32 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u32 = end.0.0;
                    
                    match counter {
                        0 => {
                                                                // +-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |  W  |  W' |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(x1 == 1, "{} ? {}", x1, 1); 
                            assert!(y1 == 0, "{} ? {}", y1, 0);  
                        },
                        _ => unreachable!("Work iterator iterated to far!"),
                    }
                    counter += 1;
                }
                assert!(counter == 1, "{} ? {}", counter, 2);
            }
           
            #[test]
            fn grid2x1_step1_work3_st() {
                use super::{WorkUnit, WorkRangeIterator};
                let queue: WorkRangeIterator<2,1,1,3> = 0.into();

                let mut counter: u64 = 0;
                for work in queue {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u32 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u32 = end.0.0;
                    
                    match counter {
                        0 => {
                                                                // +-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |  W  |     |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(x1 == 0, "{} ? {}", x1, 0); 
                            assert!(y1 == 0, "{} ? {}", y1, 0);  
                        },
                        1 => {
                                                                // +-----+-----+
                            assert!(x0 == 1, "{} ? {}", x0, 1); // |     |  W  |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(x1 == 1, "{} ? {}", x1, 1); 
                            assert!(y1 == 0, "{} ? {}", y1, 0);  
                        },
                        2 => {
                            println!("{:?}", start.0);
                            println!("{:?}", end.0);
                        }
                        _ => unreachable!("Work iterator iterated to far!"),
                    }
                    counter += 1;
                }
                assert!(counter == 2, "{} ? {}", counter, 2);
            }

            #[test]
            fn grid2x2_step1_work2_st() {
                use super::{WorkUnit, WorkRangeIterator};
                let queue: WorkRangeIterator<2,2,1,2> = 0.into();

                let mut counter: u64 = 0;
                for work in queue {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u32 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u32 = end.0.0;
                    
                    match counter {
                        0 => {
                                                                // +-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |  W  |     |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(y1 == 0, "{} ? {}", y1, 0); // |     |     | 
                            assert!(x1 == 0, "{} ? {}", x1, 0); // +-----+-----+
                        },
                        1 => {
                                                                // +-----+-----+
                            assert!(x0 == 1, "{} ? {}", x0, 1); // |     |  W  |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(y1 == 0, "{} ? {}", y1, 0); // |     |     | 
                            assert!(x1 == 1, "{} ? {}", x1, 1); // +-----+-----+
                        },
                        _ => unreachable!("Work iterator iterated to far!"),
                    }
                    counter += 1;
                }
                assert!(counter == 2, "{} ? {}", counter, 2);
            }

            #[test]
            fn grid2x2_step1_work4_st() {
                use super::{WorkUnit, WorkRangeIterator};
                let queue: WorkRangeIterator<2,2,1,4> = 0.into();

                let mut counter: u64 = 0;
                for work in queue {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u32 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u32 = end.0.0;
                    
                    match counter {
                        0 => {
                                                                // +-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |  W  |     |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(y1 == 0, "{} ? {}", y1, 0); // |     |     | 
                            assert!(x1 == 0, "{} ? {}", x1, 0); // +-----+-----+
                        },
                        1 => {
                                                                // +-----+-----+
                            assert!(x0 == 1, "{} ? {}", x0, 1); // |     |  W  |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(x1 == 1, "{} ? {}", x1, 1); // |     |     | 
                            assert!(y1 == 0, "{} ? {}", y1, 0); // +-----+-----+
                        },
                        2 => {
                                                                // +-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |     |     |
                            assert!(y0 == 1, "{} ? {}", y0, 1); // +-----+-----+
                            assert!(x1 == 0, "{} ? {}", x1, 0); // |  W  |     | 
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+
                        },
                        3 => { 
                                                                // +-----+-----+
                            assert!(x0 == 1, "{} ? {}", x0, 1); // |     |     |
                            assert!(y0 == 1, "{} ? {}", y0, 1); // +-----+-----+
                            assert!(x1 == 1, "{} ? {}", x0, 1); // |     |  W  | 
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+
                        },
                        _ => unreachable!("Work iterator iterated to far!"),
                    }
                    counter += 1;
                }
                assert!(counter == 4, "{} ? {}", counter, 4);
            }

            #[test]
            fn grid2x2_step2_work4_st() {
                use super::{WorkUnit, WorkRangeIterator};
                let queue: WorkRangeIterator<2,2,2,4> = 0.into();

                let mut counter: u64 = 0;
                for work in queue {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u32 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u32 = end.0.0;
                    
                    match counter {
                        0 => {
                                                                // +-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |  W  |  W' |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+
                            assert!(x1 == 1, "{} ? {}", x1, 1); // |     |     | 
                            assert!(y1 == 0, "{} ? {}", y1, 0); // +-----+-----+
                        },
                        1 => {
                                                                // +-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |     |     |
                            assert!(y0 == 1, "{} ? {}", y0, 1); // +-----+-----+
                            assert!(x1 == 1, "{} ? {}", x1, 1); // |  W  |  W' | 
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+
                        },
                        _ => unreachable!("Work iterator iterated to far!"),
                    }
                    counter += 1;
                }
                assert!(counter == 2, "{} ? {}", counter, 2);
            }

            #[test]
            fn grid3x2_step1_work6_st() {
                use super::{WorkUnit, WorkRangeIterator};
                let queue: WorkRangeIterator<3,2,1,6> = 0.into();

                let mut counter: u64 = 0;
                for work in queue {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u32 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u32 = end.0.0;
                    
                    match counter {
                        0 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |  W  |     |     |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+-----+
                            assert!(x1 == 0, "{} ? {}", x1, 0); // |     |     |     |
                            assert!(y1 == 0, "{} ? {}", y1, 0); // +-----+-----+-----+
                        },
                        1 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 1, "{} ? {}", x0, 1); // |     |  W  |     |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+-----+
                            assert!(x1 == 1, "{} ? {}", x1, 1); // |     |     |     |
                            assert!(y1 == 0, "{} ? {}", y1, 0); // +-----+-----+-----+
                        },
                        2 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 2, "{} ? {}", x0, 2); // |     |     |  W  |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+-----+
                            assert!(x1 == 2, "{} ? {}", x1, 2); // |     |     |     |
                            assert!(y1 == 0, "{} ? {}", y1, 0); // +-----+-----+-----+
                        },
                        3 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |     |     |     |
                            assert!(y0 == 1, "{} ? {}", y0, 1); // +-----+-----+-----+
                            assert!(x1 == 0, "{} ? {}", x1, 0); // |  W  |     |     |
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+-----+
                        },
                        4 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 1, "{} ? {}", x0, 1); // |     |     |     |
                            assert!(y0 == 1, "{} ? {}", y0, 1); // +-----+-----+-----+
                            assert!(x1 == 1, "{} ? {}", x1, 1); // |     |  W  |     |
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+-----+
                        },
                        5 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 2, "{} ? {}", x0, 2); // |     |     |     |
                            assert!(y0 == 1, "{} ? {}", y0, 1); // +-----+-----+-----+
                            assert!(x1 == 2, "{} ? {}", x1, 2); // |     |     |  W  |
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+-----+
                        },
                        _ => unreachable!("Work iterator iterated to far!"),
                    }
                    counter += 1;
                }
                assert!(counter == 6, "{} ? {}", counter, 6);
            }

            #[test]
            fn grid3x2_step2_work6_st() {
                use super::{WorkUnit, WorkRangeIterator};
                let queue: WorkRangeIterator<3,2,2,6> = 0.into();

                let mut counter: u64 = 0;
                for work in queue {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u32 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u32 = end.0.0;
                    
                    match counter {
                        0 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |  W  |  W' |     |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+-----+
                            assert!(x1 == 1, "{} ? {}", x1, 1); // |     |     |     |
                            assert!(y1 == 0, "{} ? {}", y1, 0); // +-----+-----+-----+
                        },
                        1 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 2, "{} ? {}", x0, 2); // |     |     |  W  |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+-----+
                            assert!(x1 == 0, "{} ? {}", x1, 0); // |  W' |     |     |
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+-----+
                        },
                        2 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 1, "{} ? {}", x0, 1); // |     |     |     |
                            assert!(y0 == 1, "{} ? {}", y0, 1); // +-----+-----+-----+
                            assert!(x1 == 2, "{} ? {}", x1, 2); // |     |  W  |  W' |
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+-----+
                        },
                        _ => unreachable!("Work iterator iterated to far!"),
                    }
                    counter += 1;
                }
                assert!(counter == 3, "{} ? {}", counter, 3);
            }

            #[test]
            fn grid3x2_step4_work6_st() {
                use super::{WorkUnit, WorkRangeIterator};
                let queue: WorkRangeIterator<3,2,4,6> = 0.into();

                let mut counter: u64 = 0;
                for work in queue {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u32 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u32 = end.0.0;
                    
                    match counter {
                        0 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 0, "{} ? {}", x0, 0); // |  W  |     |     |
                            assert!(y0 == 0, "{} ? {}", y0, 0); // +-----+-----+-----+
                            assert!(x1 == 0, "{} ? {}", x1, 0); // |  W' |     |     |
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+-----+
                        },
                        1 => {
                                                                // +-----+-----+-----+
                            assert!(x0 == 1, "{} ? {}", x0, 1); // |     |     |     |
                            assert!(y0 == 1, "{} ? {}", y0, 1); // +-----+-----+-----+
                            assert!(x1 == 2, "{} ? {}", x1, 2); // |     |  W  |  W' |
                            assert!(y1 == 1, "{} ? {}", y1, 1); // +-----+-----+-----+
                        },
                        _ => unreachable!("Work iterator iterated to far!"),
                    }
                    counter += 1;
                }
                assert!(counter == 2, "{} ? {}", counter, 2);
            }
        }
    }
}

