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

pub(crate) mod key;
pub(crate) mod section;

use crate::config::section::Section;
use crate::error::missing_section_error;
use crate::result::Result;
use std::collections::HashMap;

/// Represents a configuration object that stores various sections of
/// configuration information.
///
/// The `Config` struct is used to manage and access configuration
/// data stored in different sections.  Each section is identified by
/// a unique name and contains configuration options as key-value
/// pairs.
///
/// # Example
///
/// ```rust
/// use crate::Config;
///
/// let mut config = Config::new();
///
/// // Loads configuration from a file into a new section
/// section
///     .section_mut("database")
///     .load_from_path("database.cfg")
///     .unwrap();
///
/// // Retrieve a section from the config
/// let database_section = config.section("database").unwrap();
///
/// // Access configuration options within the section
/// let host = database_section.string("host").unwrap();
/// let port = database_section.integer("port").unwrap();
///
/// println!("Database configuration:");
/// println!("Host: {}", host);
/// println!("Port: {}", port);
/// ```
pub(crate) struct Config(HashMap<String, Section>);

impl Config {
    /// Creates a new instance of the `Config` struct.
    ///
    /// # Returns
    ///
    /// Returns a new, empty `Config` object.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Config;
    ///
    /// let config = Config::new();
    /// ```
    pub(crate) fn new() -> Self {
        Config(Default::default())
    }

    /// Retrieves a reference to a specific configuration section.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the section to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a reference to the requested
    /// section if it exists.  If the section is found, the result is
    /// `Ok(section)`. If the section does not exist, the result is an
    /// `Err` variant with a specific error message indicating the
    /// missing section.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Config;
    ///
    /// let config = Config::new();
    ///
    /// match config.section("database") {
    ///     Ok(section) => {
    ///         // Access configuration options within the section
    ///     },
    ///     Err(error) => {
    ///         println!("Error: {}", error);
    ///     }
    /// }
    /// ```
    pub(crate) fn section(&self, name: &str) -> Result<&Section> {
        self.0.get(name).ok_or_else(|| missing_section_error(name))
    }

    /// Retrieves a mutable reference to a specific configuration
    /// section or creates a new section if it does not exist.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the section to retrieve or create.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the requested section. If the
    /// section exists, it is returned.  If the section does not
    /// exist, a new section is created and returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::Config;
    ///
    /// let mut config = Config::new();
    ///
    /// let section = config.section_mut("database");
    ///
    /// // Set configuration options within the section
    /// section.set_key_value("host", "localhost");
    /// section.set_key_value("port", "5432");
    /// ```
    pub(crate) fn section_mut(&mut self, name: &str) -> &mut Section {
        self.0.entry(name.to_owned()).or_insert_with(Section::new)
    }
}
