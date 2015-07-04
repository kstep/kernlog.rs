#![feature(libc)]
#[macro_use]
extern crate log;
extern crate libc;

use std::fs::{OpenOptions, File};
use std::fmt::{Arguments, format};
use std::io::Write;
use std::sync::Mutex;
use libc::funcs::posix88::unistd;

use log::{Log, LogMetadata, LogRecord, LogLocation, LogLevel, MaxLogLevelFilter, LogLevelFilter};

pub struct KernelLog {
    kmsg: Mutex<File>
}

impl KernelLog {
    pub fn new() -> KernelLog {
        KernelLog {
            kmsg: Mutex::new(OpenOptions::new().write(true).open("/dev/kmsg").unwrap())
        }
    }

    pub fn init(filter: MaxLogLevelFilter) -> Box<Log> {
        filter.set(LogLevelFilter::Trace);
        Box::new(KernelLog::new())
    }
}

impl Log for KernelLog {
    fn enabled(&self, meta: &LogMetadata) -> bool {
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
        writeln!(buf, "<{}>{}[{}]: {}", level, record.target(), pid, record.args());

        if let Ok(mut kmsg) = self.kmsg.lock() {
            let _ = kmsg.write(&buf);
            kmsg.flush();
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
