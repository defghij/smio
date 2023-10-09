pub mod scribe {

    use aio_rs::aio::{
        aio_setup,
        aio_destroy,
        AioRequest,
        AioContext,
    };

    pub struct Scribble(AioContext);
    impl Scribble {
        pub fn new() -> Scribble {
            unimplemented!("Not yet");

        }
        fn set_tag(self, tag: u64) -> Self {
            unimplemented!("Not yet");
        }
    }

    #[derive(Debug, Copy, Clone)]
    pub struct Scribe<const N: usize> {
        context: AioContext,
        threshold: u32,
        pending:     [Option<AioRequest>; N],
        completed:   [Option<AioRequest>; N], // Should be io_event, actually make another type common to both.
    } impl<const N: usize>  Scribe<N> {
        pub fn new() -> Scribe<N> {
              let threshold: u32 = N as u32;
              let mut ctx: AioContext = AioContext::new();
              let ret = aio_setup(threshold, &mut ctx);
              if !ctx.is_valid() || ret.is_err() { panic!("Failed to create aio context: {}",ret.unwrap_err()); }
              Scribe::<N> {
                  context: ctx,
                  threshold,
                  pending: [None; N],
                  completed: [None; N]
              }
        }
        pub fn destroy(&mut self) {
            aio_destroy(&mut self.context);
        }
    } impl<const N: usize> std::io::Write for Scribe<N> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            unimplemented!("Not yet");
        }
        fn flush(&mut self) -> std::io::Result<()> {
            unimplemented!("Not yet");
        }

    }
}


