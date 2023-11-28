pub mod scheduler { 
    use std::sync::{
        atomic::{Ordering, AtomicU64}
    };

    /// X = PAGES
    /// Y = FILES

    #[allow(dead_code)]
    #[inline(always)]
    fn get_grid_element<const X: usize, const S: usize>(i: u64) -> (u64, u64) {
            let x0: u64 = i.rem_euclid(X as u64);
            let y0: u64 = i / (X as u64);
            (x0, y0)
    }

    pub struct WorkUnit( pub (u64, u64) );
    pub struct WorkUnitIterator<const X: usize, const Y: usize>{
        current: WorkUnit,
        stop: WorkUnit,
    }
    impl<const X: usize, const Y: usize> WorkUnitIterator<X,Y>{
        pub fn new(start: WorkUnit, stop: WorkUnit) -> WorkUnitIterator<X,Y> {

            WorkUnitIterator {
                current: start,
                stop:    stop,
            }
        }
        #[allow(dead_code)]
        pub fn next(&mut self) -> Option<(u64, u64)> {
            let x0 = self.current.0.1;
            let y0 = self.current.0.0;
            let ex = self.stop.0.1;
            let ey = self.stop.0.0;
            let current = (X as u64) * (y0) + x0;
            let stop = (X as u64) * (ey) + ex;

            if stop < current {
                return None;
            }
            let result: (u64, u64) = (y0, x0);

            let x1 = (current+1).rem_euclid(X as u64); // page mod PAGE_COUNT
            let y1: u64;
            if x1 != 0 {
                y1 = y0
            } else {
                y1 = y0 + 1;
            }
            self.current.0.0 = y1;
            self.current.0.1 = x1;
            
            Some(result)
        }
    }

    mod work_unit_iterator_tests {
        #[test]
        fn grid1x2() {
            use super::{WorkUnit,WorkUnitIterator};
            let start: WorkUnit = WorkUnit((0,0));
            let stop: WorkUnit = WorkUnit((1,0));
            let mut work_iter: WorkUnitIterator<1,2> = WorkUnitIterator::new(start, stop);

            let mut counter: u64 = 0;

            while let Some(work) = work_iter.next() {
                match counter {
                    0 => { assert!(work.0 == 0 && work.1 == 0) },
                    1 => { assert!(work.0 == 1 && work.1 == 0) },
                    _ => unreachable!("Iterated too far"),
                }
                counter +=1;
            }
            assert!(counter == 2, "{} ? {}", counter, 2);
        }
        
        #[test]
        fn grid2x1() {
            use super::{WorkUnit,WorkUnitIterator};
            let start: WorkUnit = WorkUnit((0,0));
            let stop: WorkUnit = WorkUnit((0,1));
            let mut work_iter: WorkUnitIterator<2,1> = WorkUnitIterator::new(start, stop);

            let mut counter: u64 = 0;

            while let Some(work) = work_iter.next() {
                match counter {
                    0 => { assert!(work.0 == 0 && work.1 == 0) },
                    1 => { assert!(work.0 == 0 && work.1 == 1) },
                    _ => unreachable!("Iterated too far"),
                }
                counter +=1;
            }
            assert!(counter == 2, "{} ? {}", counter, 2);
        }

        #[test]
        fn grid2x2() {
            use super::{WorkUnit,WorkUnitIterator};
            let start: WorkUnit = WorkUnit((0,0));
            let stop: WorkUnit = WorkUnit((1,1));
            let mut work_iter: WorkUnitIterator<2,2> = WorkUnitIterator::new(start, stop);

            let mut counter: u64 = 0;

            while let Some(work) = work_iter.next() {
                match counter {
                    0 => { assert!(work.0 == 0 && work.1 == 0) },
                    1 => { assert!(work.0 == 0 && work.1 == 1) },
                    2 => { assert!(work.0 == 1 && work.1 == 0) },
                    3 => { assert!(work.0 == 1 && work.1 == 1) },
                    _ => unreachable!("Iterated too far"),
                }
                counter +=1;
            }
            assert!(counter == 4, "{} ? {}", counter, 4);
        }
        #[test]
        fn grid2x3() {
            use super::{WorkUnit,WorkUnitIterator};
            let start: WorkUnit = WorkUnit((0,0));
            let stop: WorkUnit = WorkUnit((2,1));
            let mut work_iter: WorkUnitIterator<2,3> = WorkUnitIterator::new(start, stop);

            let mut counter: u64 = 0;

            while let Some(work) = work_iter.next() {
                match counter {
                    0 => { assert!(work.0 == 0 && work.1 == 0) },
                    1 => { assert!(work.0 == 0 && work.1 == 1) },
                    2 => { assert!(work.0 == 1 && work.1 == 0) },
                    3 => { assert!(work.0 == 1 && work.1 == 1) },
                    4 => { assert!(work.0 == 2 && work.1 == 0) },
                    5 => { assert!(work.0 == 2 && work.1 == 1) },
                    _ => unreachable!("Iterated too far"),
                }
                counter +=1;
            }
            assert!(counter == 6, "{} ? {}", counter, 6);
        }
        
    }

    /// This is a thread-safe compile-time sized queue which can be handed
    /// out to threaded to pull work from. The type takes three const generic
    /// type arguments: X, Y, and S. X and Y dictate the structure of the 
    /// queue while step denotes the amount of work to pull off the queue.
    ///
    /// Three cases, since start <= stop:
    ///      1. (start, stop) -> (WorkUnit, WorkUnit)
    ///      2. (start, stop] -> (WorkUnit, WorkUnit)
    ///      3. [start, stop] -> None
    ///
    /// Note that the ranges are always inclusive. In the second case, the stop
    /// value exceeds maximum work X*Y. As such, the iterator will yield a 
    /// start WorkUnit and the end WorkUnit will be the final possible work. All
    /// calls after will fall in the the third case.
    ///
    /// # Example
    /// ```ignore
    ///  let mut handles = vec![];
    ///  let queue: WorkQueueIterator<2,1,1> = 0.into();
    ///  let queue: Arc<WorkQueueIterator<2,1,1>> = Arc::new(queue);
    ///
    ///  for _ in 0..4 {
    ///    let thread_queue = queue.clone();
    /// 
    ///    let handle = thread::spawn(move || {
    ///      while let Some(work) = thread_queue.next() {
    ///        let start: WorkUnit = work.0;
    ///        let end: WorkUnit = work.1;
    ///        // Use start & end to bound some task.
    ///        // Do work...
    ///      }
    ///    });
    ///    handles.push(handle);
    ///  }
    ///
    ///  for handle in handles {
    ///    handle.join().unwrap();
    ///  }
    /// ```
    pub struct WorkQueueIterator<const X: usize, const Y: usize, const S: usize>(AtomicU64);
    impl<const X: usize, const Y: usize, const S: usize> WorkQueueIterator<X,Y,S> {
            
        #[allow(dead_code)]
        pub fn next(&self) -> Option<(WorkUnit, WorkUnit)> { //TODO, should this return a
                                                             //WorkUnitIterator??
            let x: u64 = X as u64;
            let y: u64 = Y as u64;
            let s: u64 = S as u64;
            let i: u64 = self.0.fetch_add(s, Ordering::SeqCst);

            if i >= (x*y) { 
                return None;
            }

            let (x0,y0): (u64, u64) = get_grid_element::<X,S>(i);
            let start: WorkUnit = WorkUnit( (y0 , x0) );

            let (x1,y1): (u64, u64) = get_grid_element::<X,S>( (i + s) - 1);

            let end: WorkUnit;

            if y1 < y { 
                end = WorkUnit( (y1 , x1));
            } else { // end = EOQ
                end = WorkUnit( ((y-1) , (x - 1)));
            }
            Some( (start, end) )
        }
    } impl<const X: usize, const Y: usize, const S: usize> From<u64> for WorkQueueIterator<X,Y,S> {
        fn from(item: u64) -> WorkQueueIterator<X,Y,S> {
            WorkQueueIterator(AtomicU64::new(item))
        }
    } 
    unsafe impl<const X: usize, const Y: usize, const S: usize> Send for WorkQueueIterator<X,Y,S> {}
    unsafe impl<const X: usize, const Y: usize, const S: usize> Sync for WorkQueueIterator<X,Y,S> {}

    mod work_queue_iterator_tests {

        mod single_threaded {
            #[test]
            fn grid2x1_step1() {
                use super::super::{WorkUnit, WorkQueueIterator};
                let queue: WorkQueueIterator<2,1,1> = 0.into();

                let mut counter: u64 = 0;
                while let Some(work) = queue.next() {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u64 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u64 = end.0.0;
                    
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
            fn grid2x1_step2() {
                use super::super::{WorkUnit, WorkQueueIterator};
                let queue: WorkQueueIterator<2,1,2> = 0.into();

                let mut counter: u64 = 0;
                while let Some(work) = queue.next() {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u64 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u64 = end.0.0;
                    
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
            fn grid3x2_step1() {
                use super::super::{WorkUnit, WorkQueueIterator};
                let queue: WorkQueueIterator<3,2,1> = 0.into();

                let mut counter: u64 = 0;
                while let Some(work) = queue.next() {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u64 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u64 = end.0.0;
                    
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
            fn grid3x2_step2() {
                use super::super::{WorkUnit, WorkQueueIterator};
                let queue: WorkQueueIterator<3,2,2> = 0.into();

                let mut counter: u64 = 0;
                while let Some(work) = queue.next() {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u64 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u64 = end.0.0;
                    
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
            fn grid3x2_step4() {
                use super::super::{WorkUnit, WorkQueueIterator};
                let queue: WorkQueueIterator<3,2,4> = 0.into();

                let mut counter: u64 = 0;
                while let Some(work) = queue.next() {

                    let start: WorkUnit = work.0;
                    let end: WorkUnit = work.1;
                    let x0: u64 = start.0.1;
                    let y0: u64 = start.0.0;
                    let x1: u64 = end.0.1;
                    let y1: u64 = end.0.0;
                    
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

        mod multi_threaded {

            /// Test with more threads than there are work units to be
            /// handed out. Note we cannot test counter value for each
            /// loop because that would lead to a race condition 
            /// between the queue and the counter. Instead, we simply
            /// test the queue with excessive threads and the counter
            /// at the very end ofthe test relying on if statements to
            /// ensure each case is accurate.
            #[test]
            fn grid2x1_step1_threads4() {
                use super::super::{WorkUnit, WorkQueueIterator};
                use std::thread;
                use std::sync::{
                    Arc, atomic::{
                        AtomicU64,
                        Ordering
                    }
                };
                let mut handles = vec![];
                let q: WorkQueueIterator<2,1,1> = 0.into();
                let queue: Arc<WorkQueueIterator<2,1,1>> = Arc::new(q);
                let counter: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
                let threads: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

                for _ in 0..4 {
                    let thread_queue = queue.clone();
                    let thread_counter = counter.clone();
                    let threads_total = threads.clone();

                    let handle = thread::spawn(move || {
                        threads_total.fetch_add(1, Ordering::SeqCst);

                        while let Some(work) = thread_queue.next() {
                            let start: WorkUnit = work.0;
                            let end: WorkUnit = work.1;
                            let x0: u64 = start.0.1;
                            let y0: u64 = start.0.0;
                            let x1: u64 = end.0.1;
                            let y1: u64 = end.0.0;

                            if x0 == 0 && y0 == 0 { // Case for first work unit pulled off
                                assert!(x1 == 0);   // the queue
                                assert!(y1 == 0);
                            
                            } else
                            if x0 == 1 && y0 == 0 { // Case for the second work unit
                                assert!(x1 == 1);   // pulled off the queue
                                assert!(y1 == 0);
                            } else {
                                assert!(false);    // Anything else is an error!
                            }
                            thread_counter.fetch_add(1, Ordering::SeqCst);
                        }
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.join().unwrap();
                }
            
                let counter: u64 = counter.load(Ordering::SeqCst);
                let threads: u64 = threads.load(Ordering::SeqCst);
                assert!(counter == 2, "{} ? {}", counter, 2);
                assert!(threads == 4, "{} ? {}", threads, 4);
            }
        }
    }
}

