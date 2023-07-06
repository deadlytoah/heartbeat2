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

use crate::error::{config_format_error, missing_key_error};
use crate::keyword::Keyword;
use crate::plist::KeywordPlist;
use crate::plist::{Indicator, Value};
use crate::result::Result;
use sexp::Sexp;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use super::key;

/// The name of the section configuring Heartbeat2 application.
pub(crate) static HEARTBEAT: &str = "heartbeat";

/// The name of the section configuration the sup service
pub(crate) static SUP: &str = "sup";

/// Represents a configuration section within a `Config` object.
///
/// The `Section` struct is used to store configuration options as
/// key-value pairs within a specific section of a `Config`
/// object. Each section is identified by a unique name and contains
/// configuration options represented by indicators (keys) and
/// corresponding values.
pub(crate) struct Section(HashMap<Indicator, Value>);

impl Section {
    /// Creates a new instance of the `Section` struct.
    ///
    /// # Returns
    ///
    /// Returns a new, empty `Section` object.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Section;
    ///
    /// let section = Section::new();
    /// ```
    pub(crate) fn new() -> Self {
        Section(Default::default())
    }

    /// Loads configuration data into the section from a file located
    /// at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file containing configuration data.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating the success or failure of
    /// loading the configuration data.  If the configuration is
    /// loaded successfully, the result is `Ok(())`.  If an error
    /// occurs during loading, an `Err` variant is returned with a
    /// specific error message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Section;
    ///
    /// let mut section = Section::new();
    ///
    /// section.load_from_path("config.ini").unwrap();
    /// ```
    pub(crate) fn load_from_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        *self = Self::from_file(path)?;
        Ok(())
    }

    /// Loads a `Section` object from a file located at the specified
    /// path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file containing the section data.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the loaded `Section` object if
    /// the file is read and parsed successfully.  If there is an
    /// error reading or parsing the file, an `Err` variant is
    /// returned with a specific error message.  Errors returned may
    /// either be an IO error or a SEXP error parsing the
    /// configuration file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Section;
    ///
    /// let section = Section::from_file("config.ini").unwrap();
    /// ```
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let mut buf = String::new();
        let _ = file.read_to_string(&mut buf)?;
        Self::from_sexp(sexp::parse(&buf)?)
    }

    /// Looks up the key HEARTBEAT-TIMEOUT and returns its value.
    pub(crate) fn heartbeat_timeout(&self) -> Result<u64> {
        self.0
            .get(&Indicator::new("HEARTBEAT-TIMEOUT"))
            .ok_or_else(|| missing_key_error("HEARTBEAT-TIMEOUT"))
            .and_then(Value::integer)
            .map(|v| v as u64)
    }

    /// Looks up the key TARGET-ID and returns its value.
    pub(crate) fn target_id(&self) -> Result<&Keyword> {
        self.0
            .get(&Indicator::new("TARGET-ID"))
            .ok_or_else(|| missing_key_error("TARGET-ID"))
            .and_then(Value::keyword)
    }

    /// Looks up the key TARGET-ENDPOINT and returns its value.
    pub(crate) fn target_endpoint(&self) -> Result<&str> {
        self.0
            .get(&Indicator::new(key::TARGET_ENDPOINT))
            .ok_or_else(|| missing_key_error(key::TARGET_ENDPOINT))
            .and_then(Value::string)
    }

    /// Retrieves the value associated with the specified `key` as an
    /// integer.
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the configuration option.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the integer value associated
    /// with the `key` if it exists.  If the key is found and its
    /// value can be converted to an integer, the result is
    /// `Ok(value)`.  If the key does not exist or the value cannot be
    /// converted to an integer, an `Err` variant is returned with a
    /// specific error message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Section;
    ///
    /// let section = Section::from_file("heartbeat.cfg").unwrap();
    ///
    /// match section.integer("timeout") {
    ///     Ok(value) => {
    ///         // Use the integer value
    ///     },
    ///     Err(error) => {
    ///         println!("Error: {}", error);
    ///     }
    /// }
    /// ```
    pub(crate) fn integer(&self, key: &str) -> Result<i64> {
        self.0
            .get(&Indicator::new(key))
            .ok_or_else(|| missing_key_error(key))
            .and_then(Value::integer)
    }

    /// Retrieves the value associated with the specified `key` as a
    /// list of strings.
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the configuration option.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of strings associated
    /// with the `key` if it exists.  If the key is found and its
    /// value can be converted to a list of strings, the result is
    /// `Ok(value)`.  If the key does not exist or the value cannot be
    /// converted to a list of strings, an `Err` variant is returned
    /// with a specific error message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Section;
    ///
    /// let section = Section::from_file("heartbeat.cfg").unwrap();
    ///
    /// match section.string_list("command") {
    ///     Ok(values) => {
    ///         // Use the list of strings
    ///     },
    ///     Err(error) => {
    ///         println!("Error: {}", error);
    ///     }
    /// }
    /// ```
    pub(crate) fn string_list(&self, key: &str) -> Result<Vec<String>> {
        self.0
            .get(&Indicator::new(key))
            .ok_or_else(|| missing_key_error(key))
            .and_then(Value::string_list)
    }

    /// Retrieves the value associated with the specified `key` as a
    /// string reference.
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the configuration option.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a string reference associated
    /// with the `key` if it exists.  If the key is found and its
    /// value can be represented as a string reference, the result is
    /// `Ok(value)`.  If the key does not exist or the value cannot be
    /// represented as a string reference, an `Err` variant is
    /// returned with a specific error message.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Section;
    ///
    /// let section = Section::from_file("heartbeat.cfg").unwrap();
    ///
    /// match section.string("working-directory") {
    ///     Ok(value) => {
    ///         // Use the string value
    ///     },
    ///     Err(error) => {
    ///         println!("Error: {}", error);
    ///     }
    /// }
    /// ```
    pub(crate) fn string(&self, key: &str) -> Result<&str> {
        self.0
            .get(&Indicator::new(key))
            .ok_or_else(|| missing_key_error(key))
            .and_then(Value::string)
    }

    /// Checks if the section contains a specific configuration option
    /// key.
    ///
    /// # Arguments
    ///
    /// * `key_name` - The name of the key to check.
    ///
    /// # Returns
    ///
    /// Returns `true` if the section contains the specified
    /// `key_name`, indicating that the configuration option is
    /// present. Otherwise, returns `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Section;
    ///
    /// let section = Section::from_file("heartbeat.cfg").unwrap();
    ///
    /// if section.has_key("timeout") {
    ///     println!("The 'timeout' configuration option is present.");
    /// } else {
    ///     println!("The 'timeout' configuration option is not present.");
    /// }
    /// ```
    pub(crate) fn has_key(&self, key_name: &str) -> bool {
        self.0.contains_key(&Indicator::new(key_name))
    }

    fn from_sexp(sexp: Sexp) -> Result<Self> {
        Ok(Section(
            Self::keyword_plist(Self::list_of_sexps(sexp)?)?.into_hash_map(),
        ))
    }

    fn list_of_sexps(sexp: Sexp) -> Result<Vec<Sexp>> {
        match sexp {
            Sexp::List(v) => Ok(v),
            _ => Err(config_format_error("unexpected configuration format")),
        }
    }

    fn keyword_plist(vec: Vec<Sexp>) -> Result<KeywordPlist> {
        KeywordPlist::from_vec(vec)
    }
}
