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
use crate::keyword::Keyword;
use crate::result::Result;
use sexp::Sexp;

/// Represents an atomic value in an S-expression configuration file.
///
/// The `Atom` enum represents an atomic value in an S-expression
/// configuration file. An atom can be of different types, including a
/// string, integer, float, or keyword. The `Atom` enum provides a way
/// to handle and work with these atomic values within the
/// configuration file.
///
/// # Example
///
/// ```rust
/// use crate::Atom;
///
/// let atom = Atom::String("Hello, World!".to_owned());
/// match atom {
///     Atom::String(value) => println!("String: {}", value),
///     Atom::Int(value) => println!("Integer: {}", value),
///     Atom::Float(value) => println!("Float: {}", value),
///     Atom::Keyword(value) => println!("Keyword: {:?}", value),
/// }
/// ```
pub(crate) enum Atom {
    /// Represents a string value in the configuration file.
    String(String),
    /// Represents an integer value in the configuration file.
    Int(i64),
    /// Represents a float value in the configuration file.
    Float(f64),
    /// Represents a keyword value in the configuration file.
    Keyword(Keyword),
}

/// Represents a list of expressions in an S-expression configuration
/// file.
///
/// The `List` type is an alias for a `Vec<Expression>`, where each
/// element of the vector is an expression within the list. In
/// S-expression notation, a list consists of multiple expressions
/// enclosed in parentheses. The `List` type provides a convenient way
/// to handle and manipulate lists of expressions within the
/// configuration file.
///
/// # Example
///
/// ```rust
/// use crate::{List, Expression, Atom};
///
/// let list: List = vec![
///     Expression::Atom(Atom::String("Hello".to_owned())),
///     Expression::Atom(Atom::String("World".to_owned())),
/// ];
///
/// for expr in list {
///     match expr {
///         Expression::Atom(atom) => println!("Atom: {:?}", atom),
///         Expression::List(_) => println!("Nested list found!"),
///     }
/// }
/// ```
pub(crate) type List = Vec<Expression>;

/// Represents an expression in an S-expression configuration file.
///
/// The `Expression` enum represents an expression within an
/// S-expression configuration file.  An expression can be either an
/// atomic value (`Atom`) or a list of expressions (`List`).  This
/// enum provides a way to model and work with the expressions within
/// the configuration file.
///
/// # Example
///
/// ```rust
/// use crate::{Expression, Atom};
///
/// let expression = Expression::Atom(Atom::Int(42));
/// match expression {
///     Expression::Atom(atom) => println!("Atomic value: {:?}", atom),
///     Expression::List(_) => println!("List expression found!"),
/// }
/// ```
pub(crate) enum Expression {
    /// Represents an atomic value within an expression.
    Atom(Atom),
    /// Represents a list of expressions within an expression.
    List(List),
}

impl Expression {
    /// Translates an S-expression object from the sexp crate into the
    /// project's internal representation.
    ///
    /// The `from_sexp` function takes an S-expression object (`sexp`)
    /// and converts it into an internal representation. The internal
    /// representation is similar to the S-expression structure but
    /// includes support for keywords. The function matches the input
    /// `sexp` and converts it into an `Expression` object, which can
    /// be either an atomic value (`Atom`) or a list of expressions
    /// (`List`).
    ///
    /// # Arguments
    ///
    /// * `sexp` - An S-expression object to be converted.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the converted `Expression` if
    /// the conversion is successful, or an error if the conversion
    /// fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::{Sexp, Expression};
    ///
    /// // Example S-expression object
    /// let sexp = Sexp::Atom(Atom::String("Hello".to_owned()));
    ///
    /// // Convert the S-expression into an internal representation
    /// let result = Expression::from_sexp(sexp);
    ///
    /// match result {
    ///     Ok(expression) => println!("Translated expression: {:?}", expression),
    ///     Err(err) => eprintln!("Translation error: {:?}", err),
    /// }
    /// ```
    pub(crate) fn from_sexp(sexp: Sexp) -> Result<Self> {
        match sexp {
            Sexp::Atom(atom) => Ok(Expression::Atom(Self::from_atom(atom)?)),
            Sexp::List(list) => Ok(Expression::List(Self::from_list(list)?)),
        }
    }

    /// Asserts the given expression to be a keyword, and returns the
    /// keyword if it really is.  Otherwise returns a type error.
    pub(crate) fn keyword(&self) -> Result<&Keyword> {
        if let Expression::Atom(Atom::Keyword(keyword)) = self {
            Ok(keyword)
        } else {
            Err(type_error("keyword"))
        }
    }

    /// Asserts the given expression to be a integer, and returns the
    /// integer if it really is.  Otherwise returns a type error.
    pub(crate) fn integer(&self) -> Result<i64> {
        if let Expression::Atom(Atom::Int(integer)) = self {
            Ok(*integer)
        } else {
            Err(type_error("integer"))
        }
    }

    /// Asserts the given expression to be a string, and returns the
    /// string if it really is.  Otherwise returns a type error.
    pub(crate) fn string(&self) -> Result<&str> {
        if let Expression::Atom(Atom::String(string)) = self {
            Ok(string)
        } else {
            Err(type_error("string"))
        }
    }

    /// Asserts the given expression to be a list of strings, and
    /// returns the list of strings if it really is.  Otherwise
    /// returns a type error.
    pub(crate) fn string_list(&self) -> Result<Vec<String>> {
        if let Expression::List(list) = self {
            let mut v = vec![];
            for expr in list {
                v.push(expr.string()?.to_owned());
            }
            Ok(v)
        } else {
            Err(type_error("string_list"))
        }
    }

    fn from_atom(atom: sexp::Atom) -> Result<Atom> {
        match atom {
            sexp::Atom::I(i) => Ok(Atom::Int(i)),
            sexp::Atom::F(f) => Ok(Atom::Float(f)),
            sexp::Atom::S(s) => {
                if let Some(name) = s.strip_prefix(':') {
                    Ok(Atom::Keyword(Keyword::new(&name.to_uppercase())))
                } else {
                    Ok(Atom::String(s))
                }
            }
        }
    }

    fn from_list(list: Vec<Sexp>) -> Result<List> {
        let mut v = vec![];
        for sexp in list {
            v.push(Self::from_sexp(sexp)?);
        }
        Ok(v)
    }
}
