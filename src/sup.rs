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

use crate::config::{key, section, Config};
use crate::error::{mapping_missing_error, unknown_response_error};
use crate::keyword::Keyword;
use crate::kw;
use crate::result::Result;
use crate::socket::SocketBuilder;
use std::rc::Rc;
use tmq::Context;

/// Acts as a proxy for Sup.
///
/// Sup is a straightforward naming service that resolves the service
/// endpoint from the service name.  This struct provides interface
/// for a program to access it.
///
/// # Examples
///
/// Create a new Sup proxy and query it for a service:
/// ```rust
/// use crate::keyword::kw;
/// use crate::sup::Sup;
///
/// let context = // Create a ZMQ context.
/// let config = // Load the SUP section of the configuration.
/// let sup = Sup::with_context(context, config);
/// let endpoint = sup.sget(kw!["logger"]).await?;
/// ```
pub(crate) struct Sup {
    context: Context,
    config: Rc<Config>,
}

impl Sup {
    /// Creates a new Sup proxy with the given ZMQ context.
    ///
    /// The provided configuration must contain the configuration for
    /// Sup under the section [`section::SUP`].  Invoking
    /// [`sget`](#method.sget) without having done so will result in
    /// an error.
    pub(crate) fn with_context(context: Context, config: Rc<Config>) -> Self {
        Sup { context, config }
    }

    /// Queries Sup to resolve the name of a service to its endpoint.
    ///
    /// # Returns
    ///
    /// Returns the endpoint address of the resolved service.
    ///
    /// # Error
    ///
    /// Raises an error if:
    /// * the configuration is missing under [`section::SUP`]; or
    /// * a required configuration item is missing.
    ///
    /// # Examples
    ///
    /// See the struct documentation.
    pub(crate) async fn sget(&self, id: &Keyword) -> Result<String> {
        let comms_timeout = self
            .config
            .section(section::SUP)?
            .integer(key::COMMS_TIMEOUT)?;
        let socket = SocketBuilder::new(self.context.clone())
            .endpoint(self.config.section(section::SUP)?.string(key::ENDPOINT)?)
            .linger(false)
            .timeout(comms_timeout.try_into()?)
            .req()
            .connect()?;
        let recv_sock = socket.send_keywords(&[kw![get], id.clone()]).await?;
        let (multipart, _) = recv_sock.recv_multipart().await?;
        if multipart[0] == kw![endpoint] {
            Ok(multipart[1].as_str().to_owned())
        } else if multipart[0] == kw![missing] && multipart[1] == kw![endpoint] {
            Err(mapping_missing_error(id.name()))
        } else {
            Err(unknown_response_error(multipart[0].as_str()))
        }
    }
}
