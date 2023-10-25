//mod scribe;
pub mod page;
pub mod scribe;

pub mod memory_ops {

    pub fn to_byte_slice<'a, S, D>(obj: &S) -> &'a D {
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