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
//! ```toml
//! [dependencies]
//! log = "0.4"
//! kernlog = "0.2"
//! ```
//!
//! ```rust
//! #[macro_use]
//! extern crate log;
//! extern crate kernlog;
//!
//! fn main() {
//!     kernlog::init().unwrap();
//!     warn!("something strange happened");
//! }
//! ```
//! Note you have to have permissions to write to `/dev/kmsg`,
//! which normal users (not root) usually don't.

#![deny(missing_docs)]

#[cfg_attr(test, macro_use)]
extern crate log;
extern crate libc;

use std::fs::{OpenOptions, File};
use std::io::{self, Write};
use std::sync::Mutex;
use std::env;

use log::Log;

/// Kernel logger implementation
pub struct KernelLog {
    kmsg: Mutex<File>,
    maxlevel: log::LevelFilter
}

impl KernelLog {
    /// Create new kernel logger
    pub fn new() -> io::Result<KernelLog> {
        KernelLog::with_level(log::LevelFilter::Trace)
    }

    /// Create new kernel logger with error level from `KERNLOG_LEVEL` environment variable
    pub fn from_env() -> io::Result<KernelLog> {
        match env::var("KERNLOG_LEVEL").map_err(|_| ()).and_then(|l| l.parse().map_err(|_| ())) {
            Ok(level) => KernelLog::with_level(level),
            Err(_) => KernelLog::new()
        }
    }

    /// Create new kernel logger with error level filter
    pub fn with_level(level: log::LevelFilter) -> io::Result<KernelLog> {
        Ok(KernelLog {
            kmsg: Mutex::new(OpenOptions::new().write(true).open("/dev/kmsg")?),
            maxlevel: level
        })
    }
}

impl Log for KernelLog {
    fn enabled(&self, meta: &log::Metadata) -> bool {
        meta.level() <= self.maxlevel
    }

    fn log(&self, record: &log::Record) {
        if record.level() > self.maxlevel {
            return;
        }

        let level: u8 = match record.level() {
            log::Level::Error => 3,
            log::Level::Warn => 4,
            log::Level::Info => 5,
            log::Level::Debug => 6,
            log::Level::Trace => 7,
        };

        let mut buf = Vec::new();
        writeln!(buf, "<{}>{}[{}]: {}", level, record.target(),
                 unsafe { libc::getpid() },
                 record.args()).unwrap();

        if let Ok(mut kmsg) = self.kmsg.lock() {
            let _ = kmsg.write(&buf);
            let _ = kmsg.flush();
        }
    }

    fn flush(&self) {}
}

/// Setup kernel logger as the default logger
pub fn init() -> Result<(), Result<io::Error, log::SetLoggerError>> {
    init_impl(KernelLog::new())
}

/// Setup kernel logger with error level from `KERNLOG_LEVEL` environment variable as the default logger
pub fn init_from_env() -> Result<(), Result<io::Error, log::SetLoggerError>> {
    init_impl(KernelLog::from_env())
}

/// Setup kernel logger with specified error level as the default logger
pub fn init_with_level(level: log::LevelFilter) -> Result<(), Result<io::Error, log::SetLoggerError>> {
    init_impl(KernelLog::with_level(level))
}

fn init_impl(klog: io::Result<KernelLog>) -> Result<(), Result<io::Error, log::SetLoggerError>> {
    let klog = klog.map_err(Ok)?;
    let level = klog.maxlevel;
    log::set_boxed_logger(Box::new(klog)).map_err(Err)?;
    log::set_max_level(level);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::init;

    #[test]
    fn log_to_kernel() {
        init().unwrap();
        debug!("hello, world!");
    }
}
