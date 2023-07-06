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

use crate::error::config_format_error;
use crate::expression::Expression;
use crate::keyword::Keyword;
use sexp::Sexp;
use std::collections::HashMap;
use std::error::Error;

/// Represents an indicator in a plist.
///
/// The `Indicator` type represents the type of an indicator in a
/// plist. In Lisp, a plist is a data structure used for storing and
/// looking up data by key. The `Indicator` serves as a key or
/// identifier for a corresponding `Value` in a plist.
///
/// # Examples
///
/// ```rust
/// use crate::plist::Indicator;
///
/// let indicator = Indicator::new("name");
/// ```
pub(crate) type Indicator = Keyword;

/// Represents a value in a plist.
///
/// The `Value` type represents the type of a value in a plist. In
/// Lisp, a plist is a data structure used for storing and looking up
/// data by key. A `Value` can be an atom or a list, and it is
/// associated with a specific `Indicator` in a plist.
///
/// # Examples
///
/// ```rust
/// use crate::Value;
///
/// let value = Value::Atom(Atom::String("John Doe".to_owned()));
/// ```
pub(crate) type Value = Expression;

trait StringAtom {
    fn is_string(&self) -> bool;
    fn to_s(&self) -> &str;
}

impl StringAtom for sexp::Atom {
    fn is_string(&self) -> bool {
        matches!(self, sexp::Atom::S(_))
    }

    fn to_s(&self) -> &str {
        if let sexp::Atom::S(s) = self {
            s
        } else {
            panic!("atom is not a string")
        }
    }
}

trait KeywordSexp {
    fn is_keyword(&self) -> bool;
}

impl KeywordSexp for Sexp {
    fn is_keyword(&self) -> bool {
        match self {
            Sexp::Atom(atom) => atom.is_string() && atom.to_s().starts_with(':'),
            Sexp::List(_) => false,
        }
    }
}

/// Represents a Lisp property list where every indicator is a
/// keyword.
///
/// A `KeywordPlist` is a specific type of property list (plist) in
/// Lisp, where each indicator is a keyword. In Lisp, a plist is a
/// data structure used for storing and retrieving data by key-value
/// pairs.  The `KeywordPlist` struct holds a list of `Indicator` and
/// `Value` pairs.  It features linear complexity for lookups.
/// `Indicator` represents the keyword as the key and `Value`
/// represents the associated value.
pub(crate) struct KeywordPlist(Vec<(Indicator, Value)>);

impl KeywordPlist {
    /// Creates a `KeywordPlist` from a vector of S-expressions.
    ///
    /// This function takes a vector of S-expressions representing the
    /// indicator-value pairs and constructs a `KeywordPlist` from
    /// it. Each pair in the vector is expected to have the indicator
    /// followed by the value.
    ///
    /// # Arguments
    ///
    /// * `vec` - A vector of S-expressions representing the
    /// indicator-value pairs.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the constructed `KeywordPlist`
    /// if successful, or an error if the format is invalid or the
    /// indicators are not keywords.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::plist::{KeywordPlist, Indicator, Value};
    /// use sexp::Sexp;
    ///
    /// let sexp = sexp::parse("(:host "localhost" :port 1337)").unwrap();
    /// let plist = KeywordPlist::from_sexp(&sexp).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The vector has an odd number of items, indicating a
    /// mismatched indicator-value pair.
    /// - The indicator is not a keyword.
    pub(crate) fn from_vec(vec: Vec<Sexp>) -> Result<Self, Box<dyn Error>> {
        let mut new_vec = vec![];
        for chunk in vec.chunks(2) {
            if chunk.len() < 2 {
                return Err(config_format_error("odd number of items"));
            } else {
                let indicator = &chunk[0];
                let value = &chunk[1];
                if indicator.is_keyword() {
                    new_vec.push((
                        Indicator::from_sexp(indicator.clone())?,
                        Value::from_sexp(value.clone())?,
                    ));
                } else {
                    return Err(config_format_error("indicator is not a keyword"));
                }
            }
        }
        Ok(KeywordPlist(new_vec))
    }

    /// Will be removed.
    pub(crate) fn into_hash_map(mut self) -> HashMap<Indicator, Value> {
        HashMap::from_iter(self.0.drain(..))
    }
}
