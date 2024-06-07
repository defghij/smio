
# Goals

The goals of this project are the following the following:
- Learn Linux filesystem IO
- Develop a file system benchmark
    - Two distinct functionalities:
        - Function One: creation of a corpus of test data (write intensive)
        - Function Two: benchmark file access patterns (read, write) with optional, deterministic mutation rate 
        
With the two goals in mind above, we shall focus on reads and writes of 'pages' to files.

The end goal is to have a benchmark that can test large size (in number and bytes) of files on local and shared file systems.

# Engines

Ideally, this benchmark will make use of different methods of file system IO: 
- pwrite
- libaio
- io_uring


# Development Plan

- [ ] The fundamental data type of the application: 'page' 
    - data type captures all non-IO operations that can be done on a read/writable unit. That is, modification pre/post IO.
- [ ] A type for a collection of pages.
    - All file IO will be done on a collection of pages: [0, n]
- [ ] Reading and writing of a collection of pages.


Things I need to do:
- [ ] Add To/From traits for the Page<WORDS> type for serde read/writing.
    - [ ] implement using ByteMuck as in [this example](https://github.com/MolotovCherry/virtual-display-rs/blob/e449630774ab2ae73db056bbf7062708cc118318/virtual-display-driver/src/edid.rs#L31C32-L31C50)
- [ ] For performance monitoring, check out: https://github.com/larksuite/perf-monitor-rs. If this is light weight and high resolution way to get performance data then it maybe better than doing it "in-house"

