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

use crate::error::type_error;
use sexp::Sexp;
use std::error::Error;
use std::fmt::{self, Display};
use tmq::Message;

/// Macro to create a keyword.
///
/// The `kw` macro is convenient for creating a `Keyword` object from
/// the given string. It creates a `Keyword` instance whose value is
/// the upper case of the given string.
///
/// # Examples
///
/// Creating a keyword using the `kw` macro:
///
/// ```rust
/// use crate::keyword::Keyword;
///
/// let my_keyword = kw!("my_keyword");
/// assert_eq!(my_keyword, Keyword::from("MY_KEYWORD"));
/// ```
#[macro_export]
macro_rules! kw {
    ($name:expr) => {
        $crate::keyword::Keyword::from(stringify!($name).to_uppercase())
    };
}

struct StringAtom(String);

impl StringAtom {
    fn value(&self) -> &str {
        &self.0
    }
}

trait AsStringAtom {
    fn string_atom(&self) -> Result<StringAtom, Box<dyn Error>>;
}

impl AsStringAtom for Sexp {
    fn string_atom(&self) -> Result<StringAtom, Box<dyn Error>> {
        if let sexp::Sexp::Atom(sexp::Atom::S(s)) = self {
            Ok(StringAtom(s.to_owned()))
        } else {
            Err(type_error("string"))
        }
    }
}

/// Represents a keyword in Lisp code.
///
/// The `Keyword` struct represents a keyword in a piece of Lisp code.
/// Keywords begin with a colon and identify specific symbols or
/// values.  In this project, the primary use of keywords is as
/// configuration keys.
///
/// # Examples
///
/// Creating a new keyword:
///
/// ```rust
/// use crate::Keyword;
///
/// let my_keyword = Keyword::new("my_keyword");
/// assert_eq!(my_keyword.name(), "my_keyword");
/// ```
///
/// Converting from an Sexp of the sexp crate:
///
/// ```rust
/// use crate::{Keyword, Sexp};
/// use std::error::Error;
///
/// fn convert_from_sexp(sexp: Sexp) -> Result<Keyword, Box<dyn Error>> {
///     Keyword::from_sexp(sexp)
/// }
/// ```
#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct Keyword(String);

impl Keyword {
    /// Creates a new `Keyword` with the specified name.
    ///
    /// The `new` function creates a new `Keyword` object with the
    /// given `name`. Please see the documentation for the [`kw`](kw)
    /// macro for a convenient way of creating a `Keyword`.
    ///
    /// # Parameters
    ///
    /// - `name`: A string slice representing the name of the keyword.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::Keyword;
    ///
    /// let my_keyword = Keyword::new("my_keyword");
    /// assert_eq!(my_keyword.name(), "my_keyword");
    /// ```
    pub(crate) fn new(name: &str) -> Self {
        Keyword(name.to_owned())
    }

    /// Converts an S-expression into a `Keyword`.
    ///
    /// The `from_sexp` function converts the provided `sexp` into a
    /// `Keyword` object. It expects the `sexp` to be a string atom
    /// that starts with a colon. If it is, `from_sexp` creates a
    /// `Keyword` containing the upper case of the string atom without
    /// the colon. Otherwise, it returns `type_error`.
    ///
    /// # Parameters
    ///
    /// - `sexp`: An instance of Sexp of the sexp crate, representing
    /// the keyword.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the converted `Keyword` if the
    /// conversion is successful, or a type error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::{Keyword, Sexp};
    /// use std::error::Error;
    ///
    /// fn convert_from_sexp(sexp: Sexp) -> Result<Keyword, Box<dyn Error>> {
    ///     Keyword::from_sexp(sexp)
    /// }
    /// ```
    pub(crate) fn from_sexp(sexp: Sexp) -> Result<Self, Box<dyn Error>> {
        let satom = sexp.string_atom()?;
        let value = satom.value();
        if let Some(name) = value.strip_prefix(':') {
            Ok(Keyword(name.to_uppercase()))
        } else {
            Err(type_error("keyword"))
        }
    }

    /// Returns the name of the keyword.
    ///
    /// # Returns
    ///
    /// Returns a string slice representing the name of the keyword.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::Keyword;
    ///
    /// let my_keyword = Keyword::new("my_keyword");
    /// assert_eq!(my_keyword.name(), "my_keyword");
    /// ```
    pub(crate) fn name(&self) -> &str {
        &self.0
    }
}

impl From<String> for Keyword {
    fn from(name: String) -> Self {
        Keyword(name)
    }
}

impl PartialEq<Message> for Keyword {
    fn eq(&self, message: &Message) -> bool {
        message.as_str().expect("string encoding error") == self.name()
    }
}

impl PartialEq<Keyword> for Message {
    fn eq(&self, message: &Keyword) -> bool {
        self.as_str().expect("string encoding error") == message.name()
    }
}

impl Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, ":{}", self.0)
    }
}
