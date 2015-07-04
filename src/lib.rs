//! Logger implementation for low level kernel log (using `/dev/kmsg`)
//!
//! Usually intended for low level implementations, like [systemd generators][1],
//! which have to use `/dev/kmsg`:
//!
//! > Since syslog is not available (see above) write log messages to /dev/kmsg instead.
//!
//! [1]: http://www.freedesktop.org/wiki/Software/systemd/Generators/
//!
//! # Examples
//! 
//! ```rust
//! extern crate log;
//! extern crate kernlog;
//! 
//! fn main() {
//!     log::set_logger(kernlog::KernelLog::init);
//!     warn!("something strange happened");
//! }
//! ```
//! Note you have to have permissions to write to `/dev/kmsg`,
//! which normal users (not root) usually don't.

#![deny(missing_docs)]
#![feature(libc)]
#[macro_use]

extern crate log;
extern crate libc;

use std::fs::{OpenOptions, File};
use std::io::Write;
use std::sync::Mutex;
use libc::funcs::posix88::unistd;

use log::{Log, LogMetadata, LogRecord, LogLevel, MaxLogLevelFilter, LogLevelFilter};

/// Kernel logger implementation
pub struct KernelLog {
    kmsg: Mutex<File>
}

impl KernelLog {
    /// Create new kernel logger
    pub fn new() -> KernelLog {
        KernelLog {
            kmsg: Mutex::new(OpenOptions::new().write(true).open("/dev/kmsg").unwrap())
        }
    }

    /// Setup new kernel logger for log framework
    pub fn init(filter: MaxLogLevelFilter) -> Box<Log> {
        filter.set(LogLevelFilter::Trace);
        Box::new(KernelLog::new())
    }
}

impl Log for KernelLog {
    fn enabled(&self, _meta: &LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &LogRecord) {
        let level: u8 = match record.level() {
            LogLevel::Error => 3,
            LogLevel::Warn => 4,
            LogLevel::Info => 5,
            LogLevel::Debug => 6,
            LogLevel::Trace => 7,
        };
        let pid = unsafe { unistd::getpid() };

        let mut buf = Vec::new();
        writeln!(buf, "<{}>{}[{}]: {}", level, record.target(), pid, record.args()).unwrap();

        if let Ok(mut kmsg) = self.kmsg.lock() {
            let _ = kmsg.write(&buf);
            let _ = kmsg.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KernelLog;
    use log;

    #[test]
    fn log_to_kernel() {
        log::set_logger(KernelLog::init);
        debug!("hello, world!");
    }
}
