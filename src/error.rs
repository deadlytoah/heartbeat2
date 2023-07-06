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

use std::fmt::{self, Display};

/// A type alias for the error type used within the crate.
///
/// The `Error` type represents an error that can occur during the
/// execution of operations within the crate. It is a boxed dynamic
/// trait object implementing the `std::error::Error` trait, allowing
/// it to be used as a general error type. This type alias is used to
/// simplify the handling and propagation of errors within the crate.
pub(crate) type Error = Box<dyn std::error::Error>;

/// Represents the possible error types within the crate.
///
/// The `ErrorType` enum defines the different types of errors that
/// can occur during the execution of operations within the
/// crate. Each variant of the enum represents a specific error
/// scenario and provides additional information as needed. The
/// `ErrorType` enum is used in conjunction with the `Error` type to
/// propagate and handle errors in a structured manner.  Some variants
/// may contain associated data, such as error messages or wrapped
/// `std::io::Error` instances.
#[derive(Debug)]
pub(crate) enum ErrorType {
    /// Error indicating a configuration format issue.
    ConfigFormat(String),
    /// Error indicating an illegal state.
    IllegalState(String),
    /// Error indicating a missing name to endpoint mapping for a
    /// service.
    MappingMissing(String),
    /// Error indicating a missing key in the configuration.
    MissingKey(String),
    /// Error indicating a missing section in the configuration.
    MissingSection(String),
    /// Error indicating that there is no running process.
    NoRunningProcess,
    /// Error indicating that the peer channel is closed for the
    /// internal MPSC communications channel.
    PeerChannelClosed,
    /// Error indicating a string encoding issue.
    StringEncoding,
    /// Error indicating a type errors processing S expressions.
    Type(String),
    /// Error indicating an unknown response received from a service.
    UnknownResponse(String),
    /// Error wrapping a `std::io::Error` instance.
    Io(std::io::Error),
}

impl Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ErrorType::*;
        match self {
            ConfigFormat(message) => write!(f, "config format error: {}", message),
            IllegalState(state) => write!(f, "illegal state [{}]", state),
            MappingMissing(id) => write!(f, "mapping missing for [{}] in Sup", id),
            MissingKey(key) => write!(f, "the key [{}] is missing in the config", key),
            MissingSection(section) => {
                write!(f, "the section [{}] is missing in the config", section)
            }
            NoRunningProcess => write!(f, "no running process"),
            PeerChannelClosed => write!(f, "peer channel is closed"),
            StringEncoding => write!(f, "invalid string encoding"),
            Type(expected) => write!(f, "type error (expected: {})", expected),
            UnknownResponse(response) => write!(f, "unknown response [{}]", response),
            Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ErrorType {}

impl From<std::io::Error> for ErrorType {
    fn from(value: std::io::Error) -> Self {
        ErrorType::Io(value)
    }
}

/// Creates a new config_format_error.
pub(crate) fn config_format_error(message: &str) -> Error {
    Box::new(ErrorType::ConfigFormat(message.to_owned()))
}

/// Creates a new illegal_state_error.
pub(crate) fn illegal_state_error(state: &str) -> Error {
    Box::new(ErrorType::IllegalState(state.to_owned()))
}

/// Creates a new mapping_missing_error.
pub(crate) fn mapping_missing_error(id: &str) -> Error {
    Box::new(ErrorType::MappingMissing(id.to_owned()))
}

/// Creates a new missing_key_error.
pub(crate) fn missing_key_error(key: &str) -> Error {
    Box::new(ErrorType::MissingKey(key.to_owned()))
}

/// Creates a new missing_section_error.
pub(crate) fn missing_section_error(section: &str) -> Error {
    Box::new(ErrorType::MissingSection(section.to_owned()))
}

/// Creates a new peer_channel_closed_error.
pub(crate) fn peer_channel_closed_error() -> Error {
    Box::new(ErrorType::PeerChannelClosed)
}

/// Creates a new string_encoding_error.
pub(crate) fn string_encoding_error() -> Error {
    Box::new(ErrorType::StringEncoding)
}

/// Creates a new type_error.
pub(crate) fn type_error(expected: &str) -> Error {
    Box::new(ErrorType::Type(expected.to_owned()))
}

/// Creates a new unknown_response_error.
pub(crate) fn unknown_response_error(response: &str) -> Error {
    Box::new(ErrorType::UnknownResponse(response.to_owned()))
}
