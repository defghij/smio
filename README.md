# Contents

- [Goals](#goals)
- [Engines](#engines)
- [Types](#types)
- [Development Plan](#development-plan)
- [Resources:](#resources)

# Goals

The goals of this project are the following the following:
- Learn Linux file-system IO
- Develop a file system benchmark as a vehicle for the previous.
    - Two distinct functionalities:
        - Creation of a corpus of test data (write intensive)
        - Benchmark file access patterns (read, write) with optional, deterministic mutation rate 
        
With the two goals in mind above, we shall focus on reads and writes of 'pages' to files. These pages are comprised of header meta-data and hashed payload. The hashed payload serves two purposes: a deterministic checksum of the 'page' and proxy/simulate some arbitrary cpu-based processing that directly affects the page's payload.

The end goal is to have a benchmark that can test large files-- count and size-- on local and shared file systems.

# Engines

Ideally, this benchmark will make use of different methods of file system IO: 
- pwrite
- libaio
- io_uring

# Types
 
The types of this project are layered to facilitate the layer of interaction at which they are expected to be used:
- `page`: Fundamental data type. A 'single unit' of a read and write. This is the level of data integrity and validation.
- `chapter`: A collection of pages suitable for serialization and batch reading and writing.
- `book`: A collection of chapters (i.e. Files). Writes/Reads are to/from a book if the size of a chapter.
- `bookcase`: A collection of books that comprise corpus of benchmark data (i.e. Directories). This could be thought of as non-runtime application state. That is everything that could be saved to a configuration file to relaunch the "same" job.

# Development Plan

- [X] The fundamental data type of the application: 'page' 
    - data type captures all non-IO operations that can be done on a read/writable unit. That is, modification pre/post IO.
- [X] A type for a collection of pages.
    - All file IO will be done on a collection of pages: [0, n]
- [X] Reading and writing of a collection of pages.
- [X] Single threaded; std write
- [ ] Single threaded; IO_Uring
- [ ] Multi-threaded; std write
- [ ] Multi-threaded; IO_Uring
- [ ] Multiple IO [back-ends](back-ends) ("engines")
- [ ] Add multi-process support (Lamellar?)


Things I need to do:
- [ ] Add To/From traits for the Page<WORDS> type for serde read/writing.
    - [ ] implement using ByteMuck as in [this example](https://github.com/MolotovCherry/virtual-display-rs/blob/e449630774ab2ae73db056bbf7062708cc118318/virtual-display-driver/src/edid.rs#L31C32-L31C50)
- [ ] For performance monitoring, check out: https://github.com/larksuite/perf-monitor-rs. If this is light weight and high resolution way to get performance data then it maybe better than doing it "in-house"

# Resources:

This section contains various reference and educational resources, as I come across them, related to this project.

File System Operations and Concepts:
- ![The Return of RWF_UNCACHED (dated 2024-12-04)](https://lwn.net/Articles/998783/): introduction for [the feature of] _uncached buffered I/O_
- ![Lustre Users Group 2024: Hybrid IO Update (dated 2024-05-25)](https://wiki.lustre.org/images/a/a0/LUG2024-Hybrid_IO_Path_Update-Farrell.pdf)

