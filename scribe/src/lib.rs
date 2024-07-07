//mod scribe;
pub mod page;
pub mod chapter;
pub mod bookcase;
pub mod secretary;



// Bookcase structure
pub const PAGE_SIZE: usize       = 4096 /*bytes*/;
pub const PAGE_COUNT: usize      = 512;
pub const PAGES_PER_CHAPTER: usize = 256;

// Page Structure
pub const DATA_SIZE: usize = PAGE_SIZE - page::Page::<0>::METADATA_BYTES /*bytes*/;
pub const WORDS: usize     = DATA_SIZE / 8;  /*u64s*/

pub type PageBytes = [u8; PAGE_SIZE];


#[cfg(test)]
mod integration_tests {
    use super::{
        PAGE_SIZE, PAGE_COUNT,
        bookcase::BookCase
    };

    const DIRECTORY_COUNT: usize = 2;
    const FILE_COUNT: usize = 2;

    #[test]
    fn create_pages_from_queue() {

        let pprefix: String = String::from("./testing");
        let dprefix: String = String::from("shelf");
        let fprefix: String = String::from("book");

        let mut bookcase: BookCase = BookCase::new(pprefix.to_owned(),
                              dprefix.to_owned(),
                              DIRECTORY_COUNT as u64,
                              fprefix.to_owned(),
                              FILE_COUNT as u64,
                              PAGE_SIZE,
                              PAGE_COUNT as u64);

        bookcase.construct().expect("Could not create test bookcase structures.");

        //thread_write(Arc::new(0.into()), bookcase.clone());
        //data_verify(Arc::new(0.into()), bookcase.clone());

    
        //bookcase.demolish().expect("Could not demolish test bookcase");
        assert!(true);
    }
}
