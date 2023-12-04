
use super::page::Page;

/// Type that allows conversion between an array of Pages and Bytes.
/// TODO: once `generic_const_ops` is stabilizes, remove the B generic parameter
///     in lieu of something like
///     PageOrBytes<P,W> wherein bytes has the type `[u8; {Page::<W>:PAGE_BYTES * P}]`
#[repr(C)]
#[derive(Clone, Copy)]
pub union PageOrBytes<const P: usize, const W: usize, const B: usize> { // P := PAGES
   pages: [Page<W>; P],
   bytes: [u8; B]
}

/// Allows the interaction with a collection (Array) of Pages as
/// either Pages or Bytes. The functions on this type wrap unsafe 
/// union accesses.
/// Constant Generic Arguments:
///  - P: Page count
///  - W: data words in a Page
///  - B: P * std::mem::size_of::<Page<W>>();
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Chapter<const P: usize, const W: usize, const B: usize> (PageOrBytes<P,W,B>);
impl<const P: usize, const W: usize, const B: usize> Chapter<P,W,B> {
    pub const PAGES: usize = P;
    pub const BYTES: usize = Page::<W>::PAGE_BYTES;


    #[allow(dead_code)]
    fn new() -> Chapter<P,W,B> {
        Chapter(PageOrBytes::<P,W,B> { bytes: [0; B] })
    }

    #[allow(dead_code)]
    fn bytes(&self) -> &[u8] {
        unsafe { &self.0.bytes }
    }

    #[allow(dead_code)]
    fn mutable_bytes(&mut self) -> &mut [u8] {
        unsafe { &mut self.0.bytes }
    }
    
    #[allow(dead_code)]
    fn pages(&self) -> &[Page<W>] {
        unsafe { &self.0.pages }
    }

    #[allow(dead_code)]
    fn mutable_pages(&mut self) -> &mut [Page<W>] {
        unsafe { &mut self.0.pages }
    }

    #[allow(dead_code)]
    fn single_page(&self, p: u64) -> &Page<W> {
        assert!(p < P as u64, "Attempted to pull page {} out of a chapter of length {}", p, P);
        unsafe { &self.0.pages[p as usize] }
    }

    #[allow(dead_code)]
    fn single_mutable_page(&mut self, p: u64) -> &mut Page<W> {
        assert!(p < P as u64, "Attempted to pull page {} out of a chapter of length {}", p, P);
        unsafe { &mut self.0.pages[p as usize] }
    }
}


#[test]
fn modify_pages_and_validate_bytes() {
    pub const SEED: u64  = 0xD7D6D5D4D3D2D1D0;
    pub const FID: u64   = 0xC7C6C5C4C3C2C1C0;
    pub const PID: u64   = 0xB7B6B5B4B3B2B1B0;
    pub const MUTS: u64  = 0x0000000000000000;
    let flat_tv  = vec![
        // Page One
        0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7,  // seed
        0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,  // file
        0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,  // page
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // mutations
        0x84, 0x08, 0x08, 0x03, 0xC4, 0x3E, 0xDF, 0xAF,  //  data
        // Page Two
        0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7,  // seed
        0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,  // file
        0xB1, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,  // page
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // mutations
        0xAB, 0x96, 0xF0, 0x6E, 0x6C, 0x81, 0x27, 0xD1,  //  data
    ];

    // Single Page in Chaper
    const P: usize = 1;
    const W: usize = 1;
    const B: usize = Page::<W>::PAGE_BYTES * P;

    let mut chapter: Chapter<P,W,B> = Chapter::<P,W,B>::new();

    chapter.single_mutable_page(0)
           .reinit(SEED,FID, PID, MUTS);

    assert!(flat_tv[0..B] == *chapter.bytes());
   

    // Two Page in a Chapter
    const P2: usize = 2;
    const B2: usize = Page::<W>::PAGE_BYTES * P2;
    let mut chapter: Chapter<P2,W,B2> = Chapter::<P2,W,B2>::new();
    chapter.mutable_pages()
           .iter_mut()
           .enumerate()
           .for_each(|(i,page):(usize, &mut Page<W>)| { page.reinit(SEED, FID, PID + (i as u64), MUTS); });

    assert!(flat_tv == chapter.bytes());
}
