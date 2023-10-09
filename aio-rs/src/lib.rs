pub mod aio{
    //Wrapper on top of the aio FFI that provide basic, safe, rust types.
    #![allow(dead_code)]
    use aio_sys::ffi::{
        // Values
        IOCB_CMD_PREAD,
        IOCB_CMD_PWRITE,
        IOCB_CMD_FSYNC,
        IOCB_CMD_FDSYNC,
        IOCB_CMD_POLL,
        IOCB_CMD_NOOP,
        IOCB_CMD_PREADV,
        IOCB_CMD_PWRITEV,

        // Types
        aio_context_t,
        iocb,
        io_event,

        // Functions
        io_setup,
        io_destroy,
        io_submit,
        io_cancel,
        io_getevents
    };
    use libc::{
        // Values
        EFAULT,
        EINVAL,
        ENOSYS,
        EAGAIN,
        ENOMEM,
        EBADF,

        // Types
        timespec, time_t
    };
    use std::{fmt, time::Duration};

    /// Creates an asynchrnous I/O context
    ///
    /// # Arguments
    ///
    /// * `max_events` - An integer, u32, denoting the maximum number of concurrent events the
    /// context should handle.
    /// * `ctx` - A new `AioContext` which will be populated with a handle to the resulting context if the
    /// operation is successful.
    ///
    /// # Examples
    ///
    /// ```
    /// use aio_rs::aio::{ AioContext, aio_setup };
    /// 
    /// // Set up an async I/O context
    /// let max_events: u32 = 1;
    /// let mut ctx: AioContext = AioContext::new();
    /// # let ret = aio_setup(max_events, &mut ctx);
    ///
    /// if ret.is_err() { panic!("Failed with error: {}", ret.unwrap()); }
    /// assert!(ctx.is_valid(), "Failed to set context to nonzero value!");
    /// ```
    pub fn aio_setup(max_events: u32, ctx: &mut AioContext) -> Result<i32, AioSysError> { 
        let ret: i32;
        let ctxp = ctx.inner_mut() as *mut u64;
        unsafe {
            ret = io_setup(max_events, ctxp);
        };
        if AioSysError::is_error(ret) {
            Err(AioSysError::from(ret))
        } else {
            Ok(ret)
        }
    }
       
    /// Will attempt all cancel outstanding asynchronous I/O operations on `ctx`. This function will block
    /// on completion of all operations that could not be canceled. Lastly, it will destory the
    /// `AioContext`. After a call to `aio_destroy`, `AioContext` is no longer valid for use with
    /// the runtime.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The `AioContext` which should be destroyed. Note, inner value is set to zero on
    /// success.
    ///
    /// # Examples
    /// ```
    /// # use aio_rs::aio::{ AioContext, aio_setup, aio_destroy };
    /// // Set up an async I/O context
    /// let nr_events: u32 = 1;
    /// let mut ctx: AioContext = AioContext::new();
    /// let ret = aio_setup(nr_events, &mut ctx);
    ///
    /// // Destroy an async I/O context 
    /// let ret = aio_destroy(&mut ctx);
    ///
    /// // Verify context destroyed
    /// if ret.is_err() { panic!("Failed to destory context: {}", ret.unwrap_err()); }
    /// assert!(!ctx.is_valid(), "Failed to set destroyed context value to zero!");
    /// ```
    pub fn aio_destroy(ctx: &mut AioContext) -> Result<(), AioSysError> { 
        let ret: i32;
        unsafe {
            ret = io_destroy(ctx.inner());
        }
        if AioSysError::is_error(ret) {
            Err(AioSysError::from(ret))
        } else {
            ctx.0 = 0;           
            Ok(())
        }
    }

    /// Submit an array of asynchronous I/O requests to the context.
    ///
    /// # Arguments 
    ///
    /// * `ctx` - The context to which the request are to be submitted.
    ///
    /// * `iocb` - The slice of Requests to be submitted.
    ///
    /// # Examples
    /// ```
    /// # use std::os::fd::IntoRawFd;
    /// # use std::fs::File;
    /// # use std::io::{Write, Read};
    /// # use aio_rs::aio::{ IoCmd, AioRequest, AioContext, aio_setup, aio_submit, aio_destroy };
    ///
    /// # const READ_SIZE: usize = 512;
    /// # let mut tmpfile: File = tempfile::tempfile().unwrap();
    /// # let fseg1: [u8; READ_SIZE] = [b'A'; READ_SIZE];
    /// # let _ = tmpfile.write_all(&fseg1);
    ///
    /// // Initialize I/O Runtime
    /// # let nr_events: u32 = 128;
    /// # let mut ctx: AioContext = AioContext::new();
    /// # let ret = aio_setup(nr_events, &mut ctx);
    /// # if ret.is_err() {
    /// #    assert!(false,"{}", format!("Error: {}",ret.unwrap_err()));
    /// # }
    ///
    /// // Setup Request.
    /// let file_descriptor = tmpfile.into_raw_fd();
    /// let file_offset: isize = 0;
    /// let request_tag: u64 = 0xAAAA;
    /// let request_code: IoCmd = IoCmd::Pread; 
    /// let mut destination_buffer: [u8; READ_SIZE] = [0; READ_SIZE];
    ///
    /// let iocb = AioRequest::new().add_fd(file_descriptor)
    ///                       .add_offset(file_offset)
    ///                       .add_tag(request_tag)
    ///                       .add_opcode(request_code)
    ///                       .add_buffer(&mut destination_buffer);
    /// let mut iocbs: [AioRequest; 1] = [iocb];
    ///
    /// // Submit I/O requests.
    /// let ret = aio_submit(ctx, &mut iocbs);
    /// if ret.is_err() { panic!("Failed to submit 2 iocbs: {}", ret.unwrap_err()); } 
    ///
    /// # let submitted = ret.unwrap();
    /// # assert!(submitted == 1, "Failed to submit iocb!");
    /// ```
    pub fn aio_submit(ctx: AioContext, requests: &mut [AioRequest]) -> Result<i32, AioSysError> {
        let ret: i32;
        let number_of_requests: i64 = requests.len().try_into().unwrap(); 
        unsafe {
            let mut pointers: Vec<*mut iocb> = Vec::with_capacity(requests.len());
            for iocb in requests.iter_mut() {
                pointers.push(iocb.inner_mut() as *mut iocb);
            }
            ret = io_submit(ctx.inner(), number_of_requests, pointers.as_mut_ptr());
        }
        if AioSysError::is_error(ret) {
            Err(AioSysError::from(ret))
        } else {
            Ok(ret)
        }
    }
   
    /// Attempts to read up to `events.len()` from the completion queue for the provided `ctx`. The
    /// minimum number of requests returned is 1, will block or timeout to do do, and the maximum 
    /// number of requests returned is `events.len()` up to the queue depth limits if the runtime
    /// set at `aio_setup`. There is an implicit timeout of 100ns.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The `AioContext` to get completed requests from.
    ///
    /// * `events` - A collection of events to with which to populate with completed get events.
    ///
    /// # Examples
    /// ```
    /// # use std::os::fd::IntoRawFd;
    /// # use std::fs::File;
    /// # use std::io::{Write, Read};
    /// # use aio_rs::aio::{ IoCmd, AioEvent, AioRequest, AioContext, aio_setup, aio_submit, aio_getevents, aio_destroy };
    /// # use aio_sys::ffi::io_event;
    /// # const READ_SIZE: usize = 512;
    /// # let mut tmpfile: File = tempfile::tempfile().unwrap();
    /// # let fseg1: [u8; READ_SIZE] = [b'A'; READ_SIZE];
    /// # let _ = tmpfile.write_all(&fseg1);
    /// # // Initialize I/O Runtime
    /// # let nr_events: u32 = 128;
    /// # let mut ctx: AioContext = AioContext::new();
    /// # let ret = aio_setup(nr_events, &mut ctx);
    /// # if ret.is_err() { panic!("Error: {}",ret.unwrap_err()); }
    /// # // Setup Request.
    /// # let file_descriptor = tmpfile.into_raw_fd();
    /// # let file_offset: isize = 0;
    /// # let request_tag: u64 = 0xAAAA;
    /// # let request_code: IoCmd = IoCmd::Pread; 
    /// # let mut destination_buffer: [u8; READ_SIZE] = [0; READ_SIZE];
    /// # let iocb = AioRequest::new().add_fd(file_descriptor)
    /// #                       .add_offset(file_offset)
    /// #                       .add_tag(request_tag)
    /// #                       .add_opcode(request_code)
    /// #                       .add_buffer(&mut destination_buffer);
    /// # let mut iocbs: [AioRequest; 1] = [iocb];
    /// // Submit I/O requests.
    /// let ret = aio_submit(ctx, &mut iocbs);
    /// # if ret.is_err() { panic!("Failed to submit iocbs: {}", ret.unwrap_err()); } 
    /// # let submitted = ret.unwrap();
    /// # assert!(submitted == 1, "Failed to submit iocb!");
    ///
    /// // Get Events from the runtime.
    /// let nevents: i64 = 1;
    /// let mut events: [AioEvent; 1] = [AioEvent::new(); 1]; // TODO: This should be using
    /// let ret = aio_getevents(ctx, &mut events);
    /// if ret.is_err() { panic!("Error: {}",ret.unwrap_err()); }
    /// 
    /// // Check for expected data.
    /// let events_returned = ret.unwrap();
    /// for i in 0..events_returned {
    ///     let ev: &AioEvent = &events[i as usize];
    ///     // Process events...
    /// #     assert!(ev.get_tag() == 0xAAAA || ev.get_tag() == 0xBBBB);
    /// }
    /// # destination_buffer.iter().for_each(|a| {
    /// #     if *a != b'A' { panic!("Invalid element found in read buffer"); }
    /// # });
    /// ```
    pub fn aio_getevents(ctx: AioContext, events: &mut [AioEvent]) -> Result<i32, AioSysError> { 
        let ret: i32; 
        let min_req: i64 = 1;
        let max_req: i64 = events.len() as i64;
        let mut timeout: timespec = timespec {
            tv_sec: 0,
            tv_nsec: 100,
        };
        unsafe{
            ret = io_getevents(ctx.inner(), min_req, max_req, events.as_mut_ptr() as *mut io_event, &mut timeout as *mut timespec);
        }
        if AioSysError::is_error(ret) {
            Err(AioSysError::from(ret))
        } else {
            Ok(ret)
        }
    }

    /// Attempts to cancel a previously submitted `AioRequest` request. If the operation was successful,
    /// the resulting `AioEvent` is returned. Otherwise, an `AioSysError` is returned. The manpage
    /// states the canceled request will be placed in the result field. This does not appear to
    /// exist.
    ///
    /// # Arguments 
    ///
    /// * `ctx` - The `AioContext` in which the request was submitted.
    ///
    /// * `request` - The `AioRequest` which was previously submitted but should be canceled.
    ///
    /// # Examples
    /// ```
    /// # use std::os::fd::IntoRawFd;
    /// # use std::fs::File;
    /// # use std::io::{Write, Read};
    /// # use aio_rs::aio::{ IoCmd, AioRequest, AioEvent, AioContext, aio_setup, aio_submit, aio_cancel, aio_destroy };
    /// # const READ_SIZE: usize = 512;
    /// # let mut tmpfile: File = tempfile::tempfile().unwrap();
    /// # let fseg1: [u8; READ_SIZE] = [b'A'; READ_SIZE];
    /// # let _ = tmpfile.write_all(&fseg1);
    /// # // Initialize I/O Runtime
    /// # let nr_events: u32 = 128;
    /// # let mut ctx: AioContext = AioContext::new();
    /// # let ret = aio_setup(nr_events, &mut ctx);
    /// # if ret.is_err() { panic!("Error: {}", ret.unwrap_err()); }
    /// # // Setup Request.
    /// # let file_descriptor = tmpfile.into_raw_fd();
    /// # let file_offset: isize = 0;
    /// # let request_tag: u64 = 0xAAAA;
    /// # let request_code: IoCmd = IoCmd::Pread; 
    /// # let mut destination_buffer: [u8; READ_SIZE] = [0; READ_SIZE];
    /// # let request = AioRequest::new().add_fd(file_descriptor)
    /// #                          .add_offset(file_offset)
    /// #                          .add_tag(request_tag)
    /// #                          .add_opcode(request_code)
    /// #                          .add_buffer(&mut destination_buffer);
    /// # let mut requests: [AioRequest; 1] = [request];
    /// // Submit I/O requests.
    /// let ret = aio_submit(ctx, &mut requests);
    /// # if ret.is_err() { panic!("Failed to submit 1 iocbs: {}", ret.unwrap()); } 
    /// # let submitted = ret.unwrap();
    /// # assert!(submitted == 1, "Failed to submit iocb!");
    ///
    /// // Cancel a request and check for failure.
    /// let result = aio_cancel(ctx, request);
    /// if result.is_err() { panic!("Failed to cancel request!"); }
    /// ```
    pub fn aio_cancel(ctx: AioContext, mut request: AioRequest) -> Result<AioRequest, AioSysError> { 
        let ret: i32;

        unsafe {
            ret = io_cancel(ctx.inner(), request.inner_mut());
        }
        if AioSysError::is_error(ret) {
            Err(AioSysError::from(ret))
        } else {
            Ok(request)
        }
    }

    /// Attempts to read up to `events.len()` from the completion queue for the provided `ctx`. The
    /// minimum number of requests returned is 1, will block or timeout to do do, and the maximum 
    /// number of requests returned is `events.len()` up to the queue depth limits if the runtime
    /// set at `aio_setup`. There is an explicit timeout of `timeout`.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The `AioContext` to get completed requests from.
    ///
    /// * `events` - A collection of events with which to populate with completed events.
    ///
    /// * `timeout` - A duration which dictates the timeout for the runtime to return a requested
    /// number of events.
    /// `
    ///
    ///
    /// # Examples
    /// ```
    /// # use std::os::fd::IntoRawFd;
    /// # use std::fs::File;
    /// # use std::io::{Write, Read};
    /// # use std::time::Duration;
    /// # use aio_rs::aio::{ IoCmd, AioEvent, AioRequest, AioContext, aio_setup, aio_submit, aio_getevents_with_timeout, aio_destroy };
    /// # use aio_sys::ffi::io_event;
    /// # const READ_SIZE: usize = 512;
    /// # let mut tmpfile: File = tempfile::tempfile().unwrap();
    /// # let fseg1: [u8; READ_SIZE] = [b'A'; READ_SIZE];
    /// # let _ = tmpfile.write_all(&fseg1);
    /// # // Initialize I/O Runtime
    /// # let nr_events: u32 = 128;
    /// # let mut ctx: AioContext = AioContext::new();
    /// # let ret = aio_setup(nr_events, &mut ctx);
    /// # if ret.is_err() {
    /// #    assert!(false,"{}", format!("Error: {:?}",ret.err()));
    /// # }
    /// #
    /// # // Setup Request.
    /// # let file_descriptor = tmpfile.into_raw_fd();
    /// # let file_offset: isize = 0;
    /// # let request_tag: u64 = 0xAAAA;
    /// # let request_code: IoCmd = IoCmd::Pread; 
    /// # let mut destination_buffer: [u8; READ_SIZE] = [0; READ_SIZE];
    /// #
    /// # let iocb = AioRequest::new().add_fd(file_descriptor)
    /// #                       .add_offset(file_offset)
    /// #                       .add_tag(request_tag)
    /// #                       .add_opcode(request_code)
    /// #                       .add_buffer(&mut destination_buffer);
    /// # let mut iocbs: [AioRequest; 1] = [iocb];
    /// #
    /// // Submit I/O requests.
    /// let ret = aio_submit(ctx, &mut iocbs);
    /// if ret.is_err() { panic!("Failed to submit iocbs: {}", ret.unwrap_err()); } 
    ///
    /// # let submitted = ret.unwrap();
    /// # assert!(submitted == 1, "Failed to submit iocb!");
    ///
    /// // Get Events from the runtime.
    /// let nevents: i64 = 1;
    /// let mut events: [AioEvent; 1] = [AioEvent::new(); 1];
    /// let mut timeout: Duration = Duration::new(1,0);
    /// let ret = aio_getevents_with_timeout(ctx, &mut events, timeout);
    /// if ret.is_err() { panic!("Error: {}",ret.unwrap_err()); }
    /// # // Check for expected data.
    /// # let events_returned = ret.unwrap();
    /// # for i in 0..events_returned {
    /// #     let ev: &AioEvent = &events[i as usize];
    /// #     assert!(ev.get_tag() == 0xAAAA || ev.get_tag() == 0xBBBB);
    /// # }
    /// # destination_buffer.iter().for_each(|a| {
    /// #     if *a != b'A' {
    /// #         assert!(false, "{}", "Invalid element found in read buffer");
    /// #     }
    /// # });
    /// ```
    pub fn aio_getevents_with_timeout(ctx: AioContext,
                                      events: &mut [AioEvent],
                                      timeout: Duration) -> Result<i32, AioSysError> { 
        let ret: i32; 
        let min_req: i64 = 1;
        let max_req: i64 = events.len() as i64;
        let mut timeout: timespec = timespec {
            tv_sec: timeout.as_secs() as time_t,
            tv_nsec: timeout.subsec_nanos() as time_t,
        };
        unsafe{
            ret = io_getevents(ctx.inner(), min_req, max_req, events.as_mut_ptr() as *mut io_event, &mut timeout as *mut _);
        }
        if AioSysError::is_error(ret) {
            Err(AioSysError::from(ret))
        } else {
            Ok(ret)
        }
    }


    #[derive(Debug, Copy, Clone)]
    pub struct AioContext(aio_context_t);
    impl AioContext {
        /// Creates a new context which can be passed to `aio_submit`.
        pub fn new () -> AioContext {
            AioContext(0)
        }
        /// Returns the validity of the context. Invalid may also mean uninitialized.
        pub fn is_valid(self) -> bool {
            self.0 != 0
        }
        fn inner(self) -> aio_context_t {
            self.0
        }
        fn inner_mut<'a>(&'a mut self) -> &'a mut aio_context_t {
            &mut self.0
        }
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub enum IoCmd {
        Pread   = IOCB_CMD_PREAD   as isize,
        Pwrite  = IOCB_CMD_PWRITE  as isize,
        Fsync   = IOCB_CMD_FSYNC   as isize,
        Fdsync  = IOCB_CMD_FDSYNC  as isize,
        Poll    = IOCB_CMD_POLL    as isize,
        Noop    = IOCB_CMD_NOOP    as isize,
        Preadv  = IOCB_CMD_PREADV  as isize,
        Pwritev = IOCB_CMD_PWRITEV as isize,
    } impl IoCmd {
        fn to_ctype(self) -> u16 {
            self as u16
        }
    }


    #[derive(Debug)]
    #[repr(isize)]
    pub enum AioSysError {
        Unkwn(isize),
        Eagain = EAGAIN as isize,
        Efault = EFAULT as isize,
        Einval = EINVAL as isize,
        Enosys = ENOSYS as isize,
        Enomem = ENOMEM as isize,
        Ebadf  =  EBADF as isize,
    } impl AioSysError {
        /// Returns whether the variant held is an error.
        fn is_error(i: i32) -> bool {
            match i {
                EAGAIN | EFAULT | EINVAL | ENOSYS | ENOMEM | EBADF => true,
                _ => false
            }
            
        }
    } impl From<i32> for AioSysError{
        /// Transform results from FFI to error variants.
        fn from(e: i32) -> AioSysError {
            match e {
                EAGAIN => AioSysError::Eagain,
                EFAULT => AioSysError::Efault,
                EINVAL => AioSysError::Einval,
                ENOSYS => AioSysError::Enosys,
                ENOMEM => AioSysError::Enomem,
                EBADF  => AioSysError::Ebadf,
                __     => AioSysError::Unkwn(e as isize),
            }
        }
    } impl fmt::Display for AioSysError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                AioSysError::Eagain   => write!(f, "Specified number of events exceeds limit of available events."),
                AioSysError::Efault   => write!(f, "Invalid pointer passed for context."),
                AioSysError::Einval   => write!(f, "Context is not initialized."),
                AioSysError::Enosys   => write!(f, "Setup is not implemented on this architecture."),
                AioSysError::Enomem   => write!(f, "Insufficient kernel resources available."),
                AioSysError::Ebadf    => write!(f, "Bad file descriptor given."),
                AioSysError::Unkwn(_) => write!(f, "Unknown error")
            }
        }
    }

    /// Builder for an asynchronous I/O request.
    #[derive(Debug, Copy, Clone)]
    pub struct AioRequest(iocb);
    impl AioRequest {
        /// Creates a new request object which can be passed to `aio_submit` or `aio_cancel`.
        pub fn new () -> AioRequest {
            AioRequest(iocb::default())
        }
        /// Add a file descriptor to the request
        pub fn add_fd(mut self, fd: i32) -> Self {
            if fd < u32::MIN as i32 { // i32 cant be greater than u32::MAX
                panic!("Invalid file descriptor, {}, passed!", fd);
            }
            self.0.aio_fildes = fd as u32;
            self
        }
        /// Specify the operation which the request will execute.
        pub fn add_opcode(mut self, opcode: IoCmd) -> Self {
            self.0.aio_lio_opcode = opcode as u16;
            self
        }
        /// Add a buffer, source or destination depending on the operation.
        pub fn add_buffer(mut self, buffer: &mut [u8]) -> Self {
            self.0.aio_buf = buffer.as_mut_ptr() as u64;
            self.0.aio_nbytes = buffer.len() as u64;
            self
        }
        /// Add an offset to be used for the operation. Default is zero.
        pub fn add_offset(mut self, offset: isize) -> Self {
            self.0.aio_offset = offset as i64;
            self
        }
        /// Add a tag to the request. May be used for record keeping. Default is zero.
        pub fn add_tag(mut self, tag: u64) -> Self {
            self.0.aio_data = tag;
            self
        }
        fn inner(self) -> iocb {
            self.0
        }
        fn inner_mut<'a>(&'a mut self) -> &'a mut iocb {
            &mut self.0
        }
    }
    
    /// Completed event returned from an asynchronous I/O request.
    ///
    /// C structure underlying the inner type has the following form:
    /// struct io_event {
    ///     __u64 data; // Data tag from the originating request.
    ///     __u64 obj;  // Originating context
    ///     __s64 res;  // Result code for the event
    ///     __s64 res2  // Secondary result
    /// }
    #[derive(Debug, Copy, Clone)]
    pub struct AioEvent(io_event);
    impl AioEvent {
        /// Create a new event object suitable for passing to `aio_getevents`
        pub fn new() -> AioEvent {
            AioEvent(io_event::default())
        }
        /// Retrieve the tag associated with the completed request.
        pub fn get_tag(self) -> u64 {
            self.0.data
        }
        fn inner(self) -> io_event {
            self.0
        }
        fn inner_mut<'a>(&'a mut self) -> &'a mut io_event {
            &mut self.0
        }
    } impl PartialEq<AioRequest> for AioEvent {
        fn eq(&self, other: &AioRequest) -> bool {
            self.0.obj == other.0.aio_data
        }   
        
    }

    #[cfg(test)]
    mod invocations{
        use std::os::fd::IntoRawFd;
        use std::fs::File;
        use std::io::{Write, Read};

        use super::{
            AioContext,
            IoCmd,
            AioEvent,
            AioRequest,
            aio_setup,
            aio_destroy, aio_submit, aio_getevents,
        };

        #[test]
        fn io_submit_two_reads_from_file() {
            const READ_SIZE: usize = 512;

            // Set up test file
            let mut tmpfile: File = tempfile::tempfile().unwrap();
            let fseg1: [u8; READ_SIZE] = [b'A'; READ_SIZE];
            let fseg2: [u8; READ_SIZE] = [b'B'; READ_SIZE];
            let _ = tmpfile.write_all(&fseg1);
            let _ = tmpfile.write_all(&fseg2);

            // Initialize I/O Runtime
            let nr_events: u32 = 128;
            let mut ctx: AioContext = AioContext::new();
            let ret = aio_setup(nr_events, &mut ctx);
            if ret.is_err() {
                assert!(false,"{}", format!("Error: {:?}",ret.err()));
            }

            // Setup I/O Control Blocks.
            let fd = tmpfile.into_raw_fd();

            let mut requests: Vec<AioRequest> = Vec::with_capacity(2);

            let mut read_buffer_one: [u8; READ_SIZE] = [0; READ_SIZE];
            let iocb_one = AioRequest::new().add_fd(fd)
                                      .add_offset(0)
                                      .add_tag(0xAAAA)
                                      .add_opcode(IoCmd::Pread)
                                      .add_buffer(&mut read_buffer_one);
            requests.push(iocb_one);

            let mut read_buffer_two: [u8; READ_SIZE] = [0; READ_SIZE];
            let iocb_two = AioRequest::new().add_fd(fd)
                                      .add_offset(READ_SIZE as isize)
                                      .add_tag(0xBBBB)
                                      .add_opcode(IoCmd::Pread)
                                      .add_buffer(&mut read_buffer_two);
            requests.push(iocb_two);

            // Submit I/O requests.
            let ret = aio_submit(ctx, &mut requests);
            if ret.is_err() { panic!("Failed to submit iocbs: {}", ret.unwrap_err()); } 

            let submitted = ret.unwrap();
            if submitted != 2 {
                assert!(false, "{}", format!("Failed to submit 2 iocbs: {:?}", submitted));
            }

            // Get Events from the runtime.
            let mut events: [AioEvent; 2] = [AioEvent::new(); 2];
            let ret = aio_getevents(ctx, &mut events);
            if ret.is_err() { panic!("Error: {}", ret.unwrap_err()); }


            // Check for expected data.
            let events_returned = ret.unwrap();
            for i in 0..events_returned {
                let ev: &AioEvent = &events[i as usize];
                assert!(ev.get_tag() == 0xAAAA || ev.get_tag() == 0xBBBB);
            }
            read_buffer_one.iter().zip(read_buffer_two.iter()).for_each(|(a,b)| {
                if *a != b'A' || *b != b'B' {
                    assert!(false, "{}", "Invalid element found in read buffer");
                }
            });

            // Tear down I/O Context
            let ret = aio_destroy(&mut ctx);
            if ret.is_err() { panic!("Error: {}", ret.unwrap_err()); }
            assert!(true);
        }

        #[test]
        fn io_submit_two_writes_to_file() {
            const WRITE_SIZE: usize = 512;
            let mut tmpfile: File = tempfile::tempfile().unwrap();

            // Initialize the I/O runtime.
            let nr_events: u32 = 128;
            let mut ctx: AioContext = AioContext::new();

            let ret = aio_setup(nr_events, &mut ctx);
            if ret.is_err() {
                assert!(false,"{}", format!("Error: {:?}",ret.err()));
            }

            // Set up I/O Control Blocks.
            let fd = tmpfile.try_clone().expect("Couldn't open file").into_raw_fd();

            let mut write_buffer_one: [u8; WRITE_SIZE] = [b'A'; WRITE_SIZE];
            let req_one = AioRequest::new().add_fd(fd)
                                      .add_tag(0xAAAA)
                                      .add_opcode(IoCmd::Pwrite)
                                      .add_buffer(&mut write_buffer_one);

            let mut write_buffer_two: [u8; WRITE_SIZE] = [b'B'; WRITE_SIZE];
            let req_two = AioRequest::new().add_fd(fd)
                                      .add_offset(WRITE_SIZE as isize)
                                      .add_tag(0xBBBB)
                                      .add_opcode(IoCmd::Pwrite)
                                      .add_buffer(&mut write_buffer_two);

            // Set up AioRequest pointer for submit call
            let mut requests: [AioRequest; 2] = [req_one, req_two];

            // Submit I/O requests to runtime.
            let ret = aio_submit(ctx, &mut requests);
            if ret.is_err() {
                assert!(false, "{}", format!("Failed to submit 2 iocbs: {}", ret.unwrap()));
            } 
            let submitted = ret.unwrap();
            if submitted != 2 {
                assert!(false, "{}", format!("Failed to submit 2 iocbs: {}", submitted));
            }

            // Get Events from the runtime.
            let mut events: [AioEvent; 2] = [AioEvent::new(); 2];
            let ret = aio_getevents(ctx, &mut events);
            if ret.is_err() { panic!("Error: {}", ret.unwrap_err()); }

            // Check that we got the right data fields back.
            for i in 0..ret.unwrap() {
                let ev: &AioEvent = &events[i as usize];
                assert!(ev.get_tag() == 0xAAAA || ev.get_tag() == 0xBBBB);
            }

            // Use std::io:Read to get data that was written to temporary file.
            let mut fseg1: [u8; WRITE_SIZE] = [0; WRITE_SIZE]; 
            tmpfile.read_exact(&mut fseg1).unwrap();

            let mut fseg2: [u8; WRITE_SIZE] = [0; WRITE_SIZE]; 
            tmpfile.read_exact(&mut fseg2).unwrap();

            // Compare data read from file with data that should have been written to file. 
            write_buffer_one.iter().zip(fseg1.iter()).for_each(|(a,b)| {
                if *a != *b {
                    assert!(false, "{}", "Invalid element found in buffer read from temporary file!");
                }
            });
            write_buffer_two.iter().zip(fseg2.iter()).for_each(|(a,b)| {
                if *a != *b {
                    assert!(false, "{}", "Invalid element found in buffer read from temporary file!");
                }
            });

            // Tear down I/O Context
            let ret = aio_destroy(&mut ctx);
            if ret.is_err() { panic!("Error: {}", ret.unwrap_err()); }
            assert!(true);
        }
    }
}


