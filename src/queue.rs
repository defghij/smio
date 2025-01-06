
pub mod work {
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    };
    use super::super::PAGE_COUNT;
    const COLUMNS: u64 = PAGE_COUNT as u64;

    /// # Description
    /// Representation of an element of a matrix. Used to uniquely
    /// identify a page,book pair. 
    ///
    /// Working with integers is some times easier to reason about.
    /// This type implements several From and Into traits for
    /// conversion between (x,y) -> z.
    ///
    /// # Rationale
    /// If we have work units {0,1, ..., 10, 11}, then
    /// we can represent these units in the following ways:
    ///           Page   A    B    C    D  
    ///                +----+----+----+----+
    ///    File X      | 0  | 1  | 2  | 3  |
    ///                +----+----+----+----+
    ///    File Y      | 4  | 5  | 6  | 7  |
    ///                +----+----+----+----+
    ///    File Z      | 8  | 9  | 10 | 11 |
    ///                +----+----+----+----+
    /// The above is the intended representation of work for this
    /// project. However, as state above, working with integers is
    /// often quite nice. So we can transform the above into a line
    /// of values thusly:
    ///
    /// +----+----+----+--- --+----+----+----+
    /// | 0  | 1  | 2  | ....  | 9  | 10 | 11 |
    /// +----+----+----+--  --+----+----+----+
    /// 
    /// Which allows us to iterate over pages and files or even jump
    /// between them. 
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct Unit {
        col: u64, // page/data
        row: u64, // book/file 
    } impl From<u64> for Unit {
        /// _Correctness_: This does not check bounds.
        fn from(val: u64) -> Unit {
            Unit{ 
                col: val % COLUMNS,
                row: val / COLUMNS,
            }
        }
    } impl From<AtomicU64> for Unit {
        /// _Correctness_: This does not check bounds.
        fn from(val: AtomicU64) -> Unit {
            let val: u64 = val.load(Ordering::Acquire);
            Unit{ 
                col: val % COLUMNS,
                row: val / COLUMNS,
            }
        }
    } impl Into<u64> for Unit {
        /// _Correctness_: This does not check bounds.
        fn into(self) -> u64 {
            self.row * COLUMNS + self.col % COLUMNS   
        }
    } impl Into<AtomicU64> for Unit {
        /// _Correctness_: This does not check bounds.
        fn into(self) -> AtomicU64 {
            AtomicU64::new(self.row * COLUMNS + self.col % COLUMNS)
        }
    } impl PartialOrd for Unit {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            let lhs: u64 = Unit::into(*self);
            let rhs: u64 = Unit::into(*other);
            Some(lhs.cmp(&rhs))
        }
    }

    pub struct Constraints {
        lower_bound: u64,
        upper_bound: u64,

        /// Maximum number of iterations this type will survive.
        /// After `(iteration == iteration_max) == true`
        iteration_max: u64,

        /// User provided stepping function for the iterator. This
        /// function will be called on every yield/iteration.
        step: Arc<dyn Fn(u64, Option<u64>, u64) -> Option<u64> + Send+Sync+'static>,
    } impl Constraints {
        pub fn new<F>(lower_bound: u64, upper_bound: u64, iteration_max: u64, step: F) -> Constraints
            where F: Fn(u64, Option<u64>, u64) -> Option<u64> + Send+Sync+'static {
            Constraints {
                lower_bound,
                upper_bound,
                iteration_max,
                step: Arc::new(step)
            }
        }

        /// Returns whether the supplied unit is contained within the
        /// constraints provided by this type's instantiation.
        pub fn contains(&self, other: Option<u64>) -> bool {
            other.is_some_and(|unit| {
                self.lower_bound <= unit && unit <= self.upper_bound
            })
        }
    }

    pub struct State {
        current: Option<u64>,
        iteration: u64,
    }

    pub struct Work {
        constraints: Constraints,

        state: Arc<Mutex<State>>
    } impl Work {
        pub fn new<F>(lower_bound: u64, upper_bound: u64, iteration_max: u64, step_function: F) -> Work
            where F: Fn(u64, Option<u64>, u64) -> Option<u64> + Send+Sync+'static
        {
            let constraints = Constraints {
                        lower_bound,
                        upper_bound,
                        iteration_max,
                        step: Arc::new(step_function),
            };
            let state = Arc::new(Mutex::new( 
                        State {
                            current: Some(0),
                            iteration: 0
                        }
            ));

            Work { constraints, state }
        }
    } impl Iterator for Work {
        type Item = u64;

        fn next(&mut self) -> Option<Self::Item> {
            let mut state = self.state.lock().expect("Work iterator lock poisoned-- panic!");


            let yielded_value: Option<u64> = state.current; // We can do this because
                                                      // we validate current <-- next
                                                      // below.

            let mut next: Option<u64> = (self.constraints.step)(self.constraints.lower_bound,
                                                                 state.current,
                                                                 self.constraints.upper_bound);
            state.iteration += 1;

            // Attempt to get a Some(_) until we exceed iterations count irrespective of
            // bounds.
            while next.is_none() && state.iteration < self.constraints.iteration_max {
                next = (self.constraints.step)(self.constraints.lower_bound, state.current, self.constraints.upper_bound);
                state.iteration += 1;
            }

            if !self.constraints.contains(next) || self.constraints.iteration_max <= state.iteration
            { next = None; }

            state.current = next; // save for next iteration

            yielded_value
        }
    }

    #[allow(unused)]
    mod testing {
        mod single_thread {
            const LOWER_BOUND: u64 = 0;
            const UPPER_BOUND: u64 = 3 /*rows*/ * 4 /*columns*/;
            const ITERATION_MAX: u64 = 12;
            use std::sync::{Mutex,Arc};
            use super::super::Work;

            #[test]
            fn monotonic_iteration() {

                // test basic serial && sequential iteration.
                let sequenial_step_func = |_lower, current: Option<u64>, _upper| {
                        let next = if current.is_some() {
                            Some(current.unwrap() + 1)
                        } else { None };
                        next
                };

                let sequential_work = Work::new(LOWER_BOUND,
                                                UPPER_BOUND,
                                                ITERATION_MAX,
                                                sequenial_step_func);

                let mut expected_value: u64 = 0;
                sequential_work.into_iter().for_each(|value| {
                    assert!(value == expected_value, "sequential: value ({}) != ({}) expected_value", value, expected_value);

                    println!("sequential: value ({}) ?= ({}) expected", value, expected_value);

                    expected_value += 1;
                });



                // Test non-sequential but monotonically increasing iterations. Not that this
                // means we'll skip some values and reach the UPPER_BOUND early.
                let arithmetic_step_func = move |_lower_bound, current: Option<u64>, _upper_bound| {
                        let next = if current.is_some() {
                            Some(2 * current.unwrap() + 3)
                        } else { None };
                        next
                };

                let arithemtic_work = Work::new(LOWER_BOUND,
                                                UPPER_BOUND,
                                                ITERATION_MAX,
                                                arithmetic_step_func);
                let mut expected_value: u64 = 0;
                arithemtic_work.into_iter().for_each(|value| {
                    assert!(value == expected_value, "artimetic: value ({}) != ({}) expected_value", value, expected_value);

                    println!("arithmetic: value ({}) ?= ({}) expected", value, expected_value);

                    expected_value = 2 * expected_value + 3;
                });
            }

            #[test]
            fn random_iteration() {
                use rand_xoshiro::rand_core::{RngCore, SeedableRng};
                use rand_xoshiro::Xoroshiro128PlusPlus;


                // ------------------------------------------------------------------------------
                // Use a random function to iterate through the work-space defined by
                // [LOWER_BOUND, UPPER_BOUND] with ITERATION_MAX `*_step_func` calls.
                // Thus for this test we cannot validate the yielded value as it may be random.
                // Rather, we only validate is exists within the constraints outlined at the top of
                // the module. 
                // ------------------------------------------------------------------------------

                let rng = Arc::new(Mutex::new(Xoroshiro128PlusPlus::seed_from_u64(0xdeadb33f)));

                let random_range_step_func = move |__lower_bound, current, upper_bound| {
                        let next = rng.lock().unwrap().next_u64() % upper_bound;
                        Some(next)
                };

                let random_bounded_work = Work::new(LOWER_BOUND.clone(),
                                                    UPPER_BOUND.clone(), 
                                                    ITERATION_MAX, 
                                                    random_range_step_func);

                let mut iteration_count: u64 = 0;
                random_bounded_work.into_iter().for_each(|value| {
                    assert!(LOWER_BOUND <= value && value <= UPPER_BOUND);

                    println!("bounded: (LOWER_BOUND) {LOWER_BOUND} <= {value} && {value} <= {UPPER_BOUND} (UPPER_BOUND)");

                    iteration_count += 1;
                });
                assert!(iteration_count == ITERATION_MAX, "count {iteration_count} != {ITERATION_MAX} max");



                // ------------------------------------------------------------------------------
                // This tests using an unbounded [0, u64::MAX] rng to generate the `next` values.
                // This should essentially mean Some(_) is never yielded. However, check that it
                // iterates at most `ITERATION_MAX` times. In actuality, we use the same seed, so
                // we can know what the sequence will be (should not depend on this).
                // ------------------------------------------------------------------------------
                let rng = Arc::new(Mutex::new(Xoroshiro128PlusPlus::seed_from_u64(0xdeadb33f)));
                let random_unbounded_step_func =  move |_lower_bound, _current, _upper_bound| {
                        let next = rng.lock().unwrap().next_u64();
                        Some(next)
                };

                let random_unbounded_work = Work::new(LOWER_BOUND.clone(),
                                                      UPPER_BOUND.clone(),
                                                      ITERATION_MAX,
                                                      random_unbounded_step_func);

                let mut iteration_count: u64 = 0;
                random_unbounded_work.into_iter().for_each(|value| {
                    assert!(LOWER_BOUND <= value && value <= UPPER_BOUND);

                    println!("unbounded: (LOWER_BOUND) {LOWER_BOUND} <= {value} && {value} <= {UPPER_BOUND} (UPPER_BOUND)");

                    iteration_count += 1;
                });
                assert!(iteration_count <= ITERATION_MAX);
            }
        }

        mod multi_thread {
            const LOWER_BOUND: u64 = 0;
            const UPPER_BOUND: u64 = 3 /*rows*/ * 4 /*columns*/;
            const ITERATION_MAX: u64 = 12;
            use std::sync::{Mutex,Arc};
            use rayon::{
                ThreadPoolBuilder,
                iter::{
                    IntoParallelIterator,
                    ParallelIterator as _
                }
            };
            use super::super::Work;

            #[test]
            fn monotonic_iteration() {

                let cpus: usize = std::thread::available_parallelism().unwrap().into();
                let pool = ThreadPoolBuilder::new().num_threads(cpus)
                                                   .build()
                                                   .unwrap();

                // test basic serial && sequential iteration.
                let sequenial_step_func = |_lower, current: Option<u64>, _upper| {
                        let next = if current.is_some() {
                            Some(current.unwrap() + 1)
                        } else { None };
                        next
                };

                let sequential_work = Work::new(LOWER_BOUND,
                                                UPPER_BOUND,
                                                ITERATION_MAX,
                                                sequenial_step_func);

                let mut expected_value: u64 = 0;
                //sequential_work.into_iter().for_each(|value| {
                //    assert!(value == expected_value, "sequential: value ({}) != ({}) expected_value", value, expected_value);

                //    println!("sequential: value ({}) ?= ({}) expected", value, expected_value);

                //    expected_value += 1;
                //});
                pool.install(|| {
                    (0..cpus).into_par_iter()
                             .for_each(|_|{
                                     todo!();
                             });
                });
            }
        }
    }
} 
