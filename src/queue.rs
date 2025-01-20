pub mod work {
    use std::{
        ops::Range,
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc, Mutex,
        }
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

    #[derive(Clone)]
    pub struct Constraints {
        range: Range<u64>,

        /// Maximum number of iterations this type will survive.
        /// Exclusive`
        iteration_max: u64,

    } impl Constraints {
        pub fn new<F>(range: Range<u64>, iteration_max: u64) -> Constraints {
            Constraints {
                range,
                iteration_max,
            }
        }

        /// Returns whether the supplied unit is contained within the
        /// constraints provided by this type's instantiation.
        #[inline(always)]
        fn contains(&self, s: &State) -> bool {
            s.current.is_some_and(|unit| {
                self.range.contains(&unit)
            })
        }

        #[inline(always)]
        fn is_live(&self, s: &State) -> bool {
             s.iteration < self.iteration_max
        }

        #[inline(always)]
        pub fn are_respected(&self, s: &State) -> bool { 
            self.contains(s) && self.is_live(s)
        }
    }

    #[derive(Clone)]
    pub struct State {
        /// The current value of the _DIter_. 
        current: Option<u64>,

        /// The current iteration/step of `step`. 
        iteration: u64,

        /// User provided stepping function for the iterator. This
        /// function will be called on every yield/iteration.
        step: Arc<dyn Fn(u64, Option<u64>, u64, u64) -> Option<u64> + Send+Sync+'static>,
    }

    /// # Description
    /// DIter is a Distributed Iterator, which is initialized with a map function and a set
    /// of constraints to bound its iteration, for use across threads ~~and processes~~. This
    /// iterator can then be passed to threads which will yield values in a thread-safe manner.
    ///
    /// A DIter is composed of three key parts:
    /// - Initial State: this is (0,0) = (current, iteration) if using DIter::new()
    /// - Constraints: There are two kinds of constraints-- Domain and Time to Live. The domain
    ///       constraint is given by a Range<u64> as supplied by lower_bound and upper_bound. The
    ///       time to live is the number of internal iterations that are allowed before the iteration
    ///       unconditionally yields None.
    /// - Mapping function: this is the closure used to determine the next value to be yielded.
    ///
    /// The `map` closure takes four arguments: lower_bound, current, upper_bound, and iteration.
    /// These expose any internal state that might be needed to make an arbitrary function over the
    /// range. This includes random value yielding.
    /// 
    /// Everything that this type does could be done using standard types and traits. However, that 
    /// would necessitate caring around extra values such as the constraints that are required by
    /// this type. The advantage of this method is in  the primary use case of this type: 
    /// multi-threaded iteration/synchronization. By specifying the map and its constraints up
    /// front you can avoid carrying around the bounds in each thread.
    ///
    ///
    /// # Examples
    ///
    /// The next two examples demonstrate arithmetic iteration:
    ///
    /// ```
    /// # use super_massive_io::queue::work::DIter;
    /// let map = move |_lower_bound, current: Option<u64>, _upper_bound, _iteration| {
    ///     match current {
    ///         Some(v) => Some(v +1),
    ///         None => None
    ///     }
    /// };
    ///
    /// let work = DIter::new(u64::MIN, u64::MAX, 1024, map);
    ///
    /// let mut expected: u64 = 0;
    /// work.into_iter()
    ///     .for_each(|((current, iteration)) | {
    ///         assert!(current == expected);
    ///         assert!(iteration == expected);
    ///
    ///         expected += 1;
    ///     });
    /// ```
    ///
    /// Using the current iteration as parameter in to calculate `current`.
    ///
    /// ```
    /// # use super_massive_io::queue::work::DIter;
    /// let map = move |_l, current: Option<u64>, _u, i| {
    ///     match current {
    ///         Some(v) => Some(i * v + 1),
    ///         None => None,
    ///     }
    /// };
    ///
    /// let work = DIter::new(u64::MIN, u64::MAX, 8, map);
    ///
    /// let mut expected: u64 = 0;
    /// work.into_iter()
    ///     .for_each(|(v, i) | {
    ///         assert_eq!(v, expected);
    ///
    ///         expected += i * v + 1;
    ///     });
    /// ```
    /// However, because the closure is provided information about constraints and current state,
    /// iterate can take on much more interesting forms. Below is an iterator that yields
    /// `iteration_max` random values from the provided range [`lower_bound`, `upper_bound`).
    ///
    /// ```
    /// # use super_massive_io::queue::work::DIter;
    /// # use rand_xoshiro::{
    /// #     Xoroshiro128PlusPlus,
    /// #     rand_core::{
    /// #         RngCore,
    /// #         SeedableRng
    /// #     }
    /// # };
    ///
    ///  let seed: u64 = 0xDEAD0A75;
    ///  let (lower_bound, upper_bound): (u64, u64) = (0, 1024);
    ///  let iteration_max: u64 = 64;
    ///
    ///  let random_bounded_map =  move |_l, _c: Option<u64>, u, i| {
    ///          let next = Xoroshiro128PlusPlus::seed_from_u64(seed + i).next_u64() % u;
    ///          Some(next)
    ///  };
    ///
    ///  let first_value: u64 = random_bounded_map(0,None,upper_bound,0).unwrap();
    ///
    ///  let work = DIter::new_with_state((first_value, 0),
    ///                                  lower_bound,
    ///                                  upper_bound,
    ///                                  iteration_max,
    ///                                  random_bounded_map);
    ///
    ///  let mut yielded: Vec<(u64,u64)> = work.into_iter().map(|(v,i)| {(v,i)}).collect();
    ///
    ///  /// Analogous results using standard iterator with map.
    ///  let mut rng = Xoroshiro128PlusPlus::seed_from_u64(seed);
    ///
    ///  let expected: Vec<(u64,u64)> = (lower_bound as usize..upper_bound as usize)
    ///                                  .into_iter()
    ///                                  .map(|i| { 
    ///                                      let next = Xoroshiro128PlusPlus::seed_from_u64(seed + i as u64)
    ///                                                    .next_u64() % upper_bound;
    ///                                      (next,i as u64)
    ///                                  }).collect();
    ///
    ///  yielded.iter().zip(&expected).map(|(a,b)| { assert!(a == b)} );
    /// ```
    ///
    /// # Internal Iteration
    ///
    /// The `iteration` value yielded by `DIter` and the number of invocations to `next` are not
    /// one-to-one. The provided step function may be called many times before yielding a value for
    /// use. By way of example, consider the following example wherein one call to `next` (from
    /// Iterator trait) coincides with two calls to the `map` function (i.e. discard odd-value, yield
    /// even value)
    ///
    /// ```
    /// use super_massive_io::queue::work::DIter;
    /// 
    /// // Yields only a single valid value...
    /// let map = move |lower_bound, current: Option<u64>, upper_bound, iteration| {
    ///     if iteration %  2 == 0 { Some(iteration) }
    ///     else { None }
    /// };
    ///
    /// let work = DIter::new(u64::MIN, u64::MAX, 1024, map);
    ///
    /// let mut enumerate_index: usize = 0;
    /// work.into_iter()
    ///     .enumerate()
    ///     .for_each(|(i, (current, iteration)) | {
    ///         assert!(iteration % 2 == 0);  // iteration is always even
    ///         assert!(i == enumerate_index); // strictly sequential
    ///         enumerate_index += 1;
    ///     });
    /// ```
    ///
    /// # Synchronization
    /// This type is a composite of two other types: `State` and `Constraints`. Constraints is 
    /// never modified. All modified fields are restricted to the `State` type.
    ///
    /// Currently, synchronization for this type is handled via a `Arc<Mutex<_>>`. Thus us is
    /// thread-safe. The lock for the mutex is acquired _only_ within the `next` function.
    /// In the case of simple `map` this synchronization should be light-weight (low
    /// contention). This will become less light-weight as the `map` is more 
    /// computationally expensive (more time in lock) or if there are _many_ calls to
    /// `map` required to find a valid value. As an example, the below would be a
    /// degenerate case:
    ///
    /// ```no_run
    /// use super_massive_io::queue::work::DIter;
    /// 
    /// // Yields only a single valid value...
    /// let map = move |lower_bound, current: Option<u64>, upper_bound, iteration| {
    ///     if iteration == (u64::MAX -1) { Some(iteration) }
    ///     else { None }
    /// };
    ///
    /// let work = DIter::new(u64::MIN, u64::MAX, u64::MAX, map);
    ///
    /// work.into_iter().for_each(|(c, i) | { /* accidental spin-lock */ });
    /// ```
    ///
    /// ## Future Improvements
    /// It is the hope of the author to extend this type to multi-process safety using Shared
    /// Memory programming.
    ///
    /// 
    /// # Validity
    ///
    /// Initial state of the iterator _must_ be in the domain of `map`. If you provide a
    /// step function in which the initial state, (0,0) by default, is not in the domain of the 
    /// step function then the iterator is invalid. Note that invalid means strictly that the
    /// yielded values are not in the domain of the supplied closure. The iterator may continue to
    /// yield values! Further, the element of the iterator may be the only invalid element. 
    ///
    /// The two below example illustrate iterator validity. In both define a `map` which
    /// has only odd values in its domain (i.e. f(x) % 2 = 1). 
    ///
    /// ```should_panic
    /// use super_massive_io::queue::work::DIter;
    /// let map = move |lower_bound, current: Option<u64>, upper_bound, iteration| {
    ///     if iteration % 2 == 1 { Some(iteration) }
    ///     else { None }
    /// };
    ///
    /// let work = DIter::new(u64::MIN, u64::MAX, 1024 /*iter max*/, map);
    ///
    /// // This will panic! First yielded value is (0,0) which is not in the domain of `map`
    /// work.into_iter().for_each(|(c, i) | { assert!(c % 2 == 1) });
    /// ```
    /// Instead, one must specific the initial state using the `new_with_state` function to ensure
    /// that the first yielded value is in the domain-- as follows.
    ///
    /// ```
    /// use super_massive_io::queue::work::DIter;
    /// let map = move |lower_bound, current: Option<u64>, upper_bound, iteration| {
    ///     if iteration % 2 == 1 { Some(iteration) }
    ///     else { None }
    /// };
    ///
    /// let work = DIter::new_with_state((1,1), u64::MIN, u64::MAX, 1024 /*iter max*/, map);
    ///
    /// // Does not panic. 
    /// work.into_iter().for_each(|(c, i) | { assert!(c % 2 == 1) });
    /// ```
    ///
    /// In some cases such as an iterator that yields random values in some range, the first value
    /// being non-random may be acceptable. 
    #[derive(Clone)]
    pub struct DIter {
        constraints: Constraints,
        state: Arc<Mutex<State>> // TODO: Should this use Atomics internally instead?
    } impl DIter {

        /// Creates a new DIter with initial state (current, iteration) = (0,0). Parameters to this
        /// function define and bound the behavior of the iteration.
        /// - `range`: the half-open range for which the iterator has valid values.
        /// - `iteration_max`: the number of invocations of `map` before the iterator is
        ///         exhausted.
        /// - `map`: closure defining how to calculate the next value (iteration n+1).
        pub fn new<F>(lower_bound: u64, upper_bound: u64, iteration_max: u64, map: F) -> DIter
            where F: Fn(u64, Option<u64>, u64, u64) -> Option<u64> + Send+Sync+'static
        {
            DIter::new_with_state((0, 0),
                                 lower_bound..upper_bound, 
                                 iteration_max,
                                 map)
        }

        /// Creates a new DIter with initial state provided in `initial_state`. Parameters to this
        /// function define and bound the behavior of the iteration.
        /// - `initial`: the first value returned by the iterator.
        /// - `range`: valid range of the iterator.
        /// - `iteration_max`: the number of invocations of `map` before the iterator is
        ///         exhausted.
        /// - `map`: closure defining how to calculate the next value (iteration n+1).
        pub fn new_with_state<F>(initial: (u64, u64), range: Range<u64>, iteration_max: u64, map: F) -> DIter
            where F: Fn(u64, Option<u64>, u64, u64) -> Option<u64> + Send+Sync+'static
        {
            let constraints = Constraints {
                        range,
                        iteration_max,
            };
            let state = Arc::new(Mutex::new( 
                        State {
                            current: Some(initial.0),
                            iteration: initial.1,
                            step: Arc::new(map),
                        }
            ));
            DIter { constraints, state }
        }
    } impl Iterator for DIter {
        type Item = (u64,u64);

        /// Implements iteration on DIter. This has the following properties:
        /// - The _current_ value is calculated in the previous iteration.
        /// - `step` function can be called multiple times per invocation of `next`.
        /// - `iteration`:
        ///     - is a purely monotonic and sequential value.
        ///     - refers only to `step` function invocation.
        ///     - corresponds to invocations of the provided `step` function.
        ///     - increments on each invocation of the `step` function.
        fn next(&mut self) -> Option<Self::Item> {
            let mut state = self.state.lock().expect("State mutex, for `advance`, poisoned-- panic!");
            let constraints = &self.constraints;

            let yielded: Option<(u64, u64)> = match state.current {
                // If we have _None_ then the previous invocation of `next`
                // exhausted the life of the iterator and there are no 
                // remaining values in the iterator.
                None => None,
                Some(work) => Some((work, state.iteration)),
            };

            loop { // Calculate the next yieldable value.
                state.iteration += 1;
                state.current = (state.step)(constraints.range.start,
                                             state.current,
                                             constraints.range.end,
                                             state.iteration);

                // We have a valid next value
                if state.current.is_some() && constraints.are_respected(&state) { break; }
                
                // iterator is exhausted
                if !constraints.is_live(&state) { state.current = None; break;}
            }
            yielded
        }
    } 
    // TODO Justify this
    unsafe impl Send for DIter {}
    // TODO Justify this
    unsafe impl Sync for DIter {}

    #[allow(unused)]
    mod testing {
        use std::sync::{
            Mutex,
            Arc,
            atomic::{
                AtomicU64,
                Ordering
            }
        };
        use rayon::{
            ThreadPoolBuilder,
            iter::{
                IntoParallelIterator,
                ParallelIterator as _
            }
        };
        use rand_xoshiro::{
            Xoroshiro128PlusPlus,
            rand_core::{
                RngCore,
                SeedableRng
            }
        };
        use super::DIter;
        const STARTING_VALUE: u64 = 0;
        const LOWER_BOUND: u64 = 0;
        const UPPER_BOUND: u64 = 3 /*rows*/ * 4 /*columns*/;
        const ITERATION_MAX: u64 = 12;
        const RNG_SEED: u64 = 0xDEADBEEF;

        /// Test the following invariants related to `DIter`:
        /// - `iteration`:
        ///     - is a purely monotonic and sequential value.
        ///     - increments on each invocation of the `step` function.
        mod single_threaded {
            use super::*;

            #[test]
            /// iteration is a purely monotonic and sequential value.
            fn iteration_is_monotonic_and_sequential() {

                let sequenial_map = |_l, current: Option<u64>, _u, _i| {
                        let next = if current.is_some() {
                            Some(current.unwrap() + 1)
                        } else { None };
                        next
                };
                let work = DIter::new(LOWER_BOUND,
                                     UPPER_BOUND,
                                     ITERATION_MAX,
                                     sequenial_map);

                let yielded: Vec<(u64,u64)> = work.into_iter().collect();
                let expected: Vec<(u64,u64)> = (0..12).into_iter().map(|i|{(i, i)}).collect();

                assert_eq!(yielded, expected);
            }

            #[test]
            ///  iteration increments on each invocation of the `step` function.
            fn iteration_is_of_map() {
                current_is_static();
                current_is_out_of_bounds();
                current_bounded_to_lower_half();
                only_odd_values();
                only_even_values();
                random_bounded();
            }

            fn current_is_static() {
                let static_step = |_l, current: Option<u64>, _u, _i| { current };
                let work = DIter::new(LOWER_BOUND,
                                      UPPER_BOUND,
                                      ITERATION_MAX,
                                      static_step);

                let yielded: Vec<(u64,u64)> = work.into_iter().collect();
                let expected: Vec<(u64,u64)> = (0..12).into_iter().map(|i|{(0, i)}).collect();

                assert_eq!(yielded, expected);
            }

            fn current_is_out_of_bounds() {

                // Check _liveness_ is independent of _bounds_
                // ---------------------------------------------------------------
                let null_step = |_l, _c, _u, _i| { Some(UPPER_BOUND) };  // UPPER_BOUND is
                                                                         // exclusive
                let work = DIter::new(LOWER_BOUND,
                                     UPPER_BOUND,
                                     ITERATION_MAX,
                                     null_step);

                let yielded: Vec<(u64,u64)> = work.into_iter().collect();
                let expected: Vec<(u64,u64)> = vec![(0, 0)];

                assert_eq!(yielded, expected);
            }

            fn current_bounded_to_lower_half() {
                let iteration_bounds_exclusive: u64 = ITERATION_MAX / 2;
                let value_last: u64 = iteration_bounds_exclusive - 1;
                
                let next = move |_l, c: Option<u64>, _u, i| {
                    if c.is_some() && i < iteration_bounds_exclusive {
                        Some(i)
                    } else { None }
                };

                let work = DIter::new(LOWER_BOUND,
                                     UPPER_BOUND,
                                     ITERATION_MAX,
                                     next);

                let expected: Vec<u64> = (0..12).into_iter()
                                                .filter(|x| { x < &iteration_bounds_exclusive })
                                                .collect();
                let yielded: Vec<u64> = work.into_iter().map(|(v,i)| {v}).collect();
                assert_eq!(yielded, expected);

            }

            fn only_odd_values() {
                let value_last_expected: u64 = match (UPPER_BOUND - 1) % 2 {
                    0 => { UPPER_BOUND - 2 },
                    1 => { UPPER_BOUND - 1 },
                    _ => { unreachable!("Only two cases for modulus 2"); }

                };

                let next = move |_l, c: Option<u64>, _u, i| {
                    if i % 2 == 1 { Some(i) } 
                    else { None }
                };

                // Must provide valid first/initial
                let work = DIter::new_with_state((1, 1),
                                                LOWER_BOUND..UPPER_BOUND,
                                                ITERATION_MAX,
                                                next);

                let expected: Vec<u64> = (0..12).into_iter().filter(|x| { x % 2 == 1 }).collect();
                let mut yielded: Vec<u64> = work.into_iter().map(|(v,i)| {v}).collect();

                assert_eq!(yielded, expected);
            }

            fn only_even_values() {
                let value_last_expected: u64 = match (UPPER_BOUND - 1) % 2 {
                    0 => { UPPER_BOUND - 1 },
                    1 => { UPPER_BOUND - 2 },
                    _ => { unreachable!("Only two cases for modulus 2"); }

                };

                let next = move |_l, c: Option<u64>, _u, i| {
                    if i % 2 == 0 {
                        Some(i)
                    } else { None }
                };

                let work = DIter::new_with_state((0, 0),
                                                LOWER_BOUND..UPPER_BOUND,
                                                ITERATION_MAX,
                                                next);

                let expected: Vec<u64> = (0..12).into_iter().filter(|x| { x % 2 == 0 }).collect();
                let mut yielded: Vec<u64> = work.into_iter().map(|(v,i)| {v}).collect();

                assert_eq!(yielded, expected);
            }

            fn random_bounded() {
                let random_bounded_map =  move |_l, _c: Option<u64>, upperbound, i| {
                        // TODO: replace this with a hasher or the like. Done need to generate/use
                        // an iterator.
                        let next = Xoroshiro128PlusPlus::seed_from_u64(RNG_SEED + i).next_u64() % upperbound;
                        Some(next)
                };

                let first_value: u64 = random_bounded_map(0,None,UPPER_BOUND,0).unwrap();

                // Provide initial state because default initial state (0,0) is not valid for
                // random iterator.
                let work = DIter::new_with_state((first_value, 0),
                                                LOWER_BOUND..UPPER_BOUND.clone(),
                                                ITERATION_MAX,
                                                random_bounded_map);

                let mut rng = Xoroshiro128PlusPlus::seed_from_u64(0xdeadb33f);

                let expected: Vec<(u64,u64)> = (LOWER_BOUND as usize..UPPER_BOUND as usize)
                                                .into_iter()
                                                .map(|i| { 
                                                    let next = Xoroshiro128PlusPlus::seed_from_u64(RNG_SEED + i as u64)
                                                                  .next_u64() % UPPER_BOUND;
                                                    (next,i as u64)
                                                }).collect();

                let mut yielded: Vec<(u64,u64)> = work.into_iter().map(|(v,i)| {(v,i)}).collect();

                assert_eq!(yielded, expected);
            }
        }

        mod multithreaded {
            use super::*;

            #[test]
            fn iteration_is_monotonic_and_sequential() {
                let cpus: usize = std::thread::available_parallelism().unwrap().into();
                let pool = ThreadPoolBuilder::new().num_threads(cpus)
                                                   .build()
                                                   .unwrap();

                let sequenial_map = |_l, current: Option<u64>, _u, _i| {
                        let next = if current.is_some() {
                            Some(current.unwrap() + 1)
                        } else { None };
                        next
                };
                let work = &DIter::new(LOWER_BOUND,
                                       UPPER_BOUND,
                                       ITERATION_MAX,
                                       sequenial_map);


                let yielded: Arc<Mutex<Vec<(u64,u64)>>> = Arc::new(Mutex::new(Vec::new()));
                let expected: Vec<(u64,u64)> = (0..12).into_iter().map(|i| {(i,i)}).collect();

                let yielded_thread = yielded.clone();

                pool.install(|| {
                    (0..cpus).into_par_iter()
                             .for_each(move |tid: usize|{
                                 let thread_work = work.clone();
                                 let yielded = yielded_thread.clone();

                                 thread_work.into_iter()
                                            .for_each(|work| {
                                                yielded.lock()
                                                       .expect("Lock poisoned")
                                                       .push(work);
                                            });
                             });
                });

                yielded.lock().expect("Lock poisoned").clone().iter().zip(&expected).enumerate().map(|(i, (a,b))| { assert!(a == b)} );
            }

            #[test]
            fn random_bounded() {
                let cpus: usize = std::thread::available_parallelism().unwrap().into();
                let pool = ThreadPoolBuilder::new().num_threads(cpus)
                                                   .build()
                                                   .unwrap();
                let random_bounded_map =  move |_l, _c: Option<u64>, upperbound, i| {
                        // TODO: replace this with a hasher or the like. Done need to generate/use
                        // an iterator.
                        let next = Xoroshiro128PlusPlus::seed_from_u64(RNG_SEED + i).next_u64() % upperbound;
                        Some(next)
                };

                let first_value: u64 = random_bounded_map(0,None,UPPER_BOUND,0).unwrap();

                // Provide initial state because default initial state (0,0) is not valid for
                // random iterator.
                let work = DIter::new_with_state((first_value, 0),
                                                (LOWER_BOUND..UPPER_BOUND.clone()),
                                                 ITERATION_MAX,
                                                 random_bounded_map);

                let mut rng = Xoroshiro128PlusPlus::seed_from_u64(0xdeadb33f);

                let yielded: Arc<Mutex<Vec<(u64,u64)>>> = Arc::new(Mutex::new(Vec::new()));
                let expected: Vec<(u64,u64)> = (LOWER_BOUND as usize..UPPER_BOUND as usize)
                                                .into_iter()
                                                .map(|i| { 
                                                    let next = Xoroshiro128PlusPlus::seed_from_u64(RNG_SEED + i as u64)
                                                                  .next_u64() % UPPER_BOUND;
                                                    (next,i as u64)
                                                }).collect();


                let yielded_thread = yielded.clone();

                pool.install(|| {
                    (0..cpus).into_par_iter()
                             .for_each(move |tid: usize|{
                                 let thread_work = work.clone();
                                 let yielded = yielded_thread.clone();

                                 thread_work.into_iter()
                                            .for_each(|work| {
                                                yielded.lock()
                                                       .expect("Lock poisoned")
                                                       .push(work);
                                            });
                             });
                });
                yielded.lock().expect("Lock poisoned!").clone().iter().zip(&expected).enumerate().map(|(i, (a,b))| { assert!(a == b)} );
            }
        }
    }
}
