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

use std::error::Error;

/// Indicates if a function call succeeded.
///
/// Most errors don’t need specific handling.  It is convenient to
/// abstract them as a [`Box`]ed generic error.  Since they occur in
/// exceptional cases, their handling doesn’t need to be very
/// efficient.  This justifies the use of [`Box`], which means an
/// extra memory allocation.  But it makes specific handling of errors
/// a bit inconvenient.  To handle a specific type of error, you can
/// make a case-by-case decision to use [`std::result::Result`]
/// instead.
pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;
