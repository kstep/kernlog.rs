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
//! kernlog = "0.3"
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
//! 
//! If compiled with nightly it can use libc feature to get process id
//! and report it into log. This feature is unavailable for stable release
//! for now. To enable nightly features, compile with `--features nightly`:
//!
//! ```toml
//! [dependencies.kernlog]
//! version = "*"
//! features = ["nightly"]
//! ```

#![deny(missing_docs)]
#![cfg_attr(feature="nightly", feature(libc))]

#[macro_use]
extern crate log;
extern crate libc;

use std::fs::{OpenOptions, File};
use std::io::{Write, self};
use std::path::Path;
use std::sync::Mutex;
use std::env;

use log::{Log, Metadata, Record, Level, LevelFilter, SetLoggerError};

/// Kernel logger implementation
pub struct KernelLog {
    kmsg: Mutex<File>,
    maxlevel: LevelFilter
}

impl KernelLog {

    const DEFAULT_DEVICE: &'static str = "/dev/kmsg";

    /// Create new kernel logger
    pub fn new() -> io::Result<KernelLog> {
        KernelLog::with_level(LevelFilter::Trace)
    }

    /// Create new kernel logger from default device with log level specificed by `KERNLOG_LEVEL` environment variable
    pub fn from_env() -> io::Result<KernelLog> {
        Self::from_env_with_device(Self::DEFAULT_DEVICE)
    }

    /// Create new kernel logger from default device with error level filter
    pub fn with_level(filter: LevelFilter) -> io::Result<KernelLog> {
        Self::with_device_and_level(Self::DEFAULT_DEVICE, filter)
    }

    /// Create new kernel logger from specific device
    pub fn with_device(device: impl AsRef<Path>) -> io::Result<KernelLog> {
        Self::with_device_and_level(device, LevelFilter::Trace)
    }

    /// Create new kernel logger from specific device with error level filter
    pub fn with_device_and_level(device: impl AsRef<Path>, filter: LevelFilter) -> io::Result<KernelLog> {
        Ok(KernelLog {
            kmsg: Mutex::new(OpenOptions::new().write(true).open(device.as_ref())?),
            maxlevel: filter
        })
    }

    /// Create new kernel logger from specific device with error level filter from `KERNLOG_LEVEL` environment variable
    pub fn from_env_with_device(device: impl AsRef<Path>) -> io::Result<KernelLog> {
        match env::var("KERNLOG_LEVEL") {
            Err(_) => KernelLog::with_device(device),
            Ok(s) => match s.parse() {
                Ok(filter) => KernelLog::with_device_and_level(device, filter),
                Err(_) => KernelLog::with_device(device),
            }
        }
    }
}

impl Log for KernelLog {
    fn enabled(&self, meta: &Metadata) -> bool {
        meta.level() <= self.maxlevel
    }

    fn log(&self, record: &Record) {
        if record.level() > self.maxlevel {
            return;
        }

        let level: u8 = match record.level() {
            Level::Error => 3,
            Level::Warn => 4,
            Level::Info => 5,
            Level::Debug => 6,
            Level::Trace => 7,
        };

        let mut buf = Vec::new();
        writeln!(buf, "<{}>{}[{}]: {}", level, record.target(),
                 unsafe { ::libc::getpid() },
                 record.args()).unwrap();

        if let Ok(mut kmsg) = self.kmsg.lock() {
            let _ = kmsg.write(&buf);
            let _ = kmsg.flush();
        }
    }

    fn flush(&self) {}
}

/// KernelLog initialization error
#[derive(Debug)]
pub enum KernelLogInitError {
    /// IO error
    Io(io::Error),
    /// Set logger error
    Log(SetLoggerError)
}

impl std::fmt::Display for KernelLogInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            KernelLogInitError::Io(err) => err.fmt(f),
            KernelLogInitError::Log(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for KernelLogInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            KernelLogInitError::Io(err) => Some(err),
            KernelLogInitError::Log(err) => Some(err),
        }
    }
}

impl From<SetLoggerError> for KernelLogInitError {
    fn from(err: SetLoggerError) -> Self {
        KernelLogInitError::Log(err)
    }
}
impl From<io::Error> for KernelLogInitError {
    fn from(err: io::Error) -> Self {
        KernelLogInitError::Io(err)
    }
}

/// Setup kernel logger as a default logger
pub fn init() -> Result<(), KernelLogInitError> {
    init_with_device(KernelLog::DEFAULT_DEVICE)
}

/// Setup kernel logger as a default logger with specific device
pub fn init_with_device(device: impl AsRef<Path>) -> Result<(), KernelLogInitError> {
    let klog = KernelLog::from_env_with_device(device)?;
    let maxlevel = klog.maxlevel;
    log::set_boxed_logger(Box::new(klog))?;
    log::set_max_level(maxlevel);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{KernelLog, init};

    #[test]
    fn log_to_kernel() {
        init().unwrap();
        debug!("hello, world!");
    }
}
