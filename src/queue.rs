use std::sync::{atomic, atomic::AtomicU64, Arc};
use std::option::Option;
use std::cmp;


pub mod work {
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    };
    use super::super::PAGE_COUNT;
    const COLUMNS: u64 = PAGE_COUNT as u64;

    ///     let page_id = work % page_count_per_book as u64;
    ///     let book_id = work / page_count_per_book as u64;
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
        lower_bound: Unit,
        upper_bound: Unit,

        /// Maximum number of iterations this type will survive.
        /// After `(iteration == iteration_max) == true`
        iteration_max: u64,

        /// User provided stepping function for the iterator. This
        /// function will be called on every yield/iteration.
        step: Arc<dyn Fn(Option<u64>) -> Option<u64> + Send+Sync+'static>,
    } impl Constraints {
        pub fn new<F>(lower_bound: Unit, upper_bound: Unit, iteration_max: u64, step: F) -> Constraints
            where F: Fn(Option<u64>) -> Option<u64> + Send+Sync+'static {
            Constraints {
                lower_bound,
                upper_bound,
                iteration_max,
                step: Arc::new(step)
            }
        }

        /// Returns whether the supplied unit is contained within the
        /// constraints provided by this type's instantiation.
        pub fn contains(&self, other: Unit) -> bool {
            self.lower_bound <= other && other <= self.upper_bound
        }
    }

    pub struct State {
        current: Option<Unit>,
        next: Option<Unit>,
        iteration: u64,
    }

    pub struct Work {
        constraints: Constraints,

        state: Arc<Mutex<State>>
    } impl Work {
        pub fn new<F>(lower_bound: Unit, upper_bound: Unit, iteration_max: u64, step: F) -> Work
            where F: Fn(Option<u64>) -> Option<u64> + Send+Sync+'static
        {
            let constraints = Constraints {
                        lower_bound,
                        upper_bound,
                        iteration_max,
                        step: Arc::new(step),
            };
            let state = Arc::new(Mutex::new( 
                        State {
                            current: Some(Unit::from(0)),
                            next: None,
                            iteration: 0
                        }
            ));

            Work { constraints, state }
        }
    } impl Iterator for Work {
        type Item = Unit;

        /// TODO: This needs to check wither current is None, act appropriately,
        ///     check the bounds if it is not None,
        ///     call the closure to calculate next.
        ///     Maybe still set next even if current is None (maybe the one after isnt?)
        fn next(&mut self) -> Option<Self::Item> {
            let mut state = self.state.lock().expect("Work iterator lock poisoned-- panic!");

            state.current = state.next;

            if state.current.is_none() { return state.current; } 

            //let curr_unit: Unit = Unit::into(state.current);

            //state.next = (self.constraints.step)(state.current);

            //if state.current <= self.constraints.lower_bound || self.constraints.upper_bound <= state.current {
            //    return None;
            //}

            state.current
        }
    }
} 

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Bounds {
    /// Inclusive Lower Bound
    pub lower: u64,

    /// Exclusive Upper Bound
    pub upper: u64
} impl PartialOrd for Bounds {
    fn partial_cmp(&self, other: &Bounds) -> Option<cmp::Ordering> { 
        todo!();
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct WorkRange {
    row: Bounds, // book/file
    col: Bounds, // page/data
}


#[derive(Clone)]
pub struct Queue {
    /// A closure or function that defines how to calculate the
    /// next `current` value.
    /// TODO: Provide a default serial monotonic implementation
    step: Arc<dyn Fn(u64) -> Option<u64> + Send+Sync+'static>,

    /// `current` is the current counter value for the queue.
    /// This value should be bounded by [lower_bound,upper_bound).
    /// where the lower bound is inclusive and the upper bound is 
    /// exclusive.
    current: Arc<AtomicU64>,

    // The start (inclusive) of the work range which queue
    // will allow as valid
    lower_bound: u64,

    // The end (exclusive) of the work range which queue
    // will allow as valid
    upper_bound: u64,
} impl Queue {
    pub fn new<F>(lower_bound: u64, upper_bound: u64, next: F) -> Self 
    where F: Fn(u64) -> Option<u64> + Send+Sync+'static {
        Self {
            step: Arc::new(next),
            current: Arc::new(AtomicU64::new(lower_bound)),
            lower_bound,
            upper_bound,
        }
    }

    pub fn take_work(&self) -> Option<u64> {
        loop {
            // Figure out if the next `current` is value given the bounds
            let current = self.current.load(atomic::Ordering::Relaxed);
            let next = (self.step)(current)?;

            if next <= self.lower_bound || self.upper_bound <= next {
                return None;
            }

            // Attempt to set the new `current` value. If fail, loop and try again.
            if self.current
                   .compare_exchange(current, next, atomic::Ordering::SeqCst, atomic::Ordering::Relaxed)
                   .is_ok() 
            {
                return Some(current);
            }
        }
    }
}
