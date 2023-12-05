use aio_rs::aio::AioContext;

pub struct IoUringContext();

pub enum Nib {
    Posix,
    AIO(AioContext),
    IOUring(IoUringContext)
}


pub struct Quill {
    nib: Nib,
}
