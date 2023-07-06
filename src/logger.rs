/*
 * Heartbeat2: Monitors & restarts software on crashes or deadlocks.
 * Copyright (C) 2022-2023  Hee Shin
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use chrono::Local;
use core::fmt::{self, Display};

/// Represents the log level for logging messages.
///
/// The `LogLevel` enum represents different levels of log
/// messages. Each log level represents a different severity and
/// verbosity of the logged message. In the order of increasing
/// severity and decreasing verbosity, they are: `Debug`, `Trace`,
/// `Info`, `Warning`, `Error`, `Severe` and `Fatal`.
///
/// # Examples
///
/// ```rust
/// use crate::LogLevel;
///
/// let logger = Logger::new("MyApp");
/// logger.log(LogLevel::Error, "Error description");
/// ```
pub enum LogLevel {
    /// Represents debug-level log messages used for debugging
    /// purposes.
    Debug,
    /// Represents trace-level log messages used for detailed tracing
    /// and debugging.
    Trace,
    /// Represents informational log messages that provide general
    /// information.
    Info,
    /// Represents log messages indicating a potential issue or
    /// warning.
    Warning,
    /// Represents log messages indicating an error occurred.
    Error,
    /// Represents log messages indicating a severe error or critical
    /// issue.
    Severe,
    /// Represents log messages indicating a fatal error that causes
    /// the application to exit.
    Fatal,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LogLevel::*;
        match self {
            Debug => write!(f, "Debug"),
            Trace => write!(f, "Trace"),
            Info => write!(f, "Info"),
            Warning => write!(f, "Warning"),
            Error => write!(f, "Error"),
            Severe => write!(f, "Severe"),
            Fatal => write!(f, "Fatal"),
        }
    }
}

/// Logs messages to a local logging destination, such as standard
/// error or a local file.
///
/// The `LocalLogger` struct provides functionality to log messages to
/// a local logging destination.  A local logging destination may be
/// standard error or a file in the local disk.  `Heartbeat2` uses
/// `LocalLogger` to record various events or messages for debugging
/// and monitoring.  `Heartbeat2` is a port of `Heartbeat`, which was
/// written in Lisp.  It inherits `LocalLogger` from `Heartbeat`.
/// `LocalLogger` goes hand in hand with `RemoteLogger`.
/// `RemoteLogger` has the same interface as `LocalLogger`.  But it
/// logs to a remote logging service instead of a local destination.
/// It relies on IPC over an asynchronous message queue to log
/// messages behind the scene.  `Heartbeat2` has `LocalLogger`
/// implemented, but not `RemoteLogger`, at the moment.
///
/// # Examples
///
/// ```rust
/// use crate::{LocalLogger, LogLevel};
///
/// let logger = LocalLogger::new("my_app");
/// logger.log(LogLevel::Info, "Initializing application");
/// ```
pub struct LocalLogger {
    app_id: String,
}

impl LocalLogger {
    /// Creates a new instance of `LocalLogger` with the specified
    /// application identifier.
    ///
    /// The new function creates a new instance of `LocalLogger` with
    /// the provided `app_id`.  The `app_id` identifies the source or
    /// context of the logged messages.  `LocalLogger` minimises
    /// visual clutter as it presents `app_id` in the log messages.
    /// Having `app_id` allows for quick and effortless visual
    /// scanning over the log messages.  It also makes it easy to use
    /// text processing tools to filter or manipulate the log
    /// messages.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::LocalLogger;
    ///
    /// let logger = LocalLogger::new("my_app");
    /// ```
    pub fn new(app_id: &str) -> Self {
        LocalLogger {
            app_id: app_id.to_owned(),
        }
    }

    /// Logs a message with the specified log level.
    ///
    /// The log function logs a message with the given level and
    /// message.  The level represents the severity of the logged
    /// message, and message is its content.  You can use
    /// [format!](format!) macro to format a log message as in the
    /// example below.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::{LocalLogger, LogLevel};
    ///
    /// let logger = LocalLogger::new("my_app");
    /// logger.log(LogLevel::Info, "Initializing application");
    /// logger.log(LogLevel::Info, &format!("Application ID: {}", "my_app"));
    /// ```
    pub fn log(&self, level: LogLevel, message: &str) {
        eprintln!(
            "[{}] [{}] {}: {}",
            self.app_id,
            Local::now(),
            level,
            message
        );
    }
}
