

pub enum ExitCode {
    OK = 0,
    Critical = 1,
    MAJOR = 10,
    MINOR = 11,
    UNDEF
}

pub enum Verbosity {
    NONE,
    INFORMATIONAL,
    DEBUG,
    WARNING
}

use std::io::Result;
use scribe::scribe::create;



fn main() -> Result<()> {
    create()
}
