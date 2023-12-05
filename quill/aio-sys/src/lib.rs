pub mod ffi {
    #![allow(dead_code)]
    extern crate std;
    extern crate libc;
    use libc::{
        c_long, 
        c_int,
        c_uint,
    };

    pub use libc::timespec;
    use std::mem::zeroed;
    use std::default::Default;

    #[allow(non_camel_case_types)]
    pub type aio_context_t = u64;

    pub const IOCB_CMD_PREAD: u32 = 0;
    pub const IOCB_CMD_PWRITE: u32 = 1;
    pub const IOCB_CMD_FSYNC: u32 = 2;
    pub const IOCB_CMD_FDSYNC: u32 = 3;
    pub const IOCB_CMD_POLL: u32 = 5;
    pub const IOCB_CMD_NOOP: u32 = 6;
    pub const IOCB_CMD_PREADV: u32 = 7;
    pub const IOCB_CMD_PWRITEV: u32 = 8;

    #[link(name = "aio")]
    extern "C" {
        pub fn io_setup(nr_events: c_uint, ctxp: *mut aio_context_t) -> c_int;
        pub fn io_destroy(ctx_id: aio_context_t) -> c_int;
        pub fn io_submit(ctx_id: aio_context_t, nr: c_long, iocbpp: *mut *mut iocb) -> c_int;
        pub fn io_cancel(ctx_id: aio_context_t, iocb: *mut iocb) -> c_int;
        pub fn io_getevents(ctx_id: aio_context_t,
                            min_nr: c_long,
                            nr: c_long,
                            events: *mut io_event,
                            timeout: *mut timespec) -> c_int;
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct iocb {
        pub aio_data: u64,
        pub aio_key: u32,
        pub aio_rw_flags: i32,
        pub aio_lio_opcode: u16,
        pub aio_reqprio: i16,
        pub aio_fildes: u32,
        pub aio_buf: u64,
        pub aio_nbytes: u64,
        pub aio_offset: i64,
        pub aio_reserved2: i64,
        pub aio_flags: u32,
        pub aio_resfd: u32,
    } impl Default for iocb {
        fn default() -> iocb {
            iocb { aio_lio_opcode: IOCB_CMD_NOOP as u16,
                          aio_fildes: (-1i32) as u32,
                          .. unsafe { zeroed() }
            }
        }
    }


    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct io_event {
        pub data: u64,
        pub obj: u64,
        pub res: i64,
        pub res2: i64,
    } impl Default for io_event {
        fn default() -> io_event {
            unsafe { zeroed() }
        }

    }

    
    #[cfg(test)]
    mod layouts {
        use super::{
            io_event,
            iocb,
        };
        use std::{
            mem::{
                MaybeUninit,
                align_of,
                size_of
            },
            ptr::{
                addr_of
            }
        };
        #[test]
        fn iocb() {
            const UNINIT: MaybeUninit<iocb> = MaybeUninit::uninit();
            let ptr = UNINIT.as_ptr();
            assert_eq!(
                size_of::<iocb>(),
                64usize,
                concat!("Size of: ", stringify!(iocb))
            );
            assert_eq!(
                align_of::<iocb>(),
                8usize,
                concat!("Alignment of ", stringify!(iocb))
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_data) as usize - ptr as usize },
                0usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_data)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_key) as usize - ptr as usize },
                8usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_key)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_rw_flags) as usize - ptr as usize },
                12usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_rw_flags)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_lio_opcode) as usize - ptr as usize },
                16usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_lio_opcode)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_reqprio) as usize - ptr as usize },
                18usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_reqprio)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_fildes) as usize - ptr as usize },
                20usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_fildes)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_buf) as usize - ptr as usize },
                24usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_buf)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_nbytes) as usize - ptr as usize },
                32usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_nbytes)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_offset) as usize - ptr as usize },
                40usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_offset)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_reserved2) as usize - ptr as usize },
                48usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_reserved2)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_flags) as usize - ptr as usize },
                56usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_flags)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).aio_resfd) as usize - ptr as usize },
                60usize,
                concat!(
                    "Offset of field: ",
                    stringify!(iocb),
                    "::",
                    stringify!(aio_resfd)
                )
            );
        }
        #[test]
        fn io_event() {
            const UNINIT: MaybeUninit<io_event> = MaybeUninit::uninit();
            let ptr = UNINIT.as_ptr();
            assert_eq!(
                size_of::<io_event>(),
                32usize,
                concat!("Size of: ", stringify!(io_event))
            );
            assert_eq!(
                align_of::<io_event>(),
                8usize,
                concat!("Alignment of ", stringify!(io_event))
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).data) as usize - ptr as usize },
                0usize,
                concat!(
                    "Offset of field: ",
                    stringify!(io_event),
                    "::",
                    stringify!(data)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).obj) as usize - ptr as usize },
                8usize,
                concat!(
                    "Offset of field: ",
                    stringify!(io_event),
                    "::",
                    stringify!(obj)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).res) as usize - ptr as usize },
                16usize,
                concat!(
                    "Offset of field: ",
                    stringify!(io_event),
                    "::",
                    stringify!(res)
                )
            );
            assert_eq!(
                unsafe { addr_of!((*ptr).res2) as usize - ptr as usize },
                24usize,
                concat!(
                    "Offset of field: ",
                    stringify!(io_event),
                    "::",
                    stringify!(res2)
                )
            );
        }

    }
}


