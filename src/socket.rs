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

use crate::error::Error;
use crate::keyword::Keyword;
use crate::result::Result;
use std::fmt::{self, Display};
use std::ops::Deref;
use tmq::request_reply::{RequestReceiver, RequestSender};
use tmq::{self, Context};
use tokio::time::Duration;

/// The default socket communications timeout in milliseconds.
static DEFAULT_SOCKET_TIMEOUT: u64 = 3000;

/// Defines a ZMQ message.
///
/// A message can be either a string or a keyword.  A keyword is an
/// upper case string that begins with a colon in the configuration
/// file.
pub(crate) enum Message {
    /// A string message.
    String(String),
    /// A keyword message.
    Keyword(Keyword),
}

impl Message {
    /// Returns the string representation of the message.
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Message::String(s) => s,
            Message::Keyword(kw) => kw.name(),
        }
    }
}

impl TryFrom<tmq::Message> for Message {
    type Error = Box<dyn std::error::Error>;

    fn try_from(source: tmq::Message) -> Result<Self> {
        source
            .as_str()
            .ok_or_else(crate::error::string_encoding_error)
            .map(|s| {
                if let Some(name) = s.strip_prefix(':') {
                    Message::Keyword(Keyword::new(name))
                } else {
                    Message::String(s.to_owned())
                }
            })
    }
}

impl PartialEq<Keyword> for Message {
    fn eq(&self, rhs: &Keyword) -> bool {
        match self {
            Message::String(s) => s == rhs.name(),
            Message::Keyword(kw) => kw == rhs,
        }
    }
}

/// Represents a ZMQ multipart message.
///
/// Contains a sequence of [`Message`]s.  ZMQ either sends the entire
/// multipart message or not at all.
pub(crate) struct Multipart(Vec<Message>);

impl TryFrom<tmq::Multipart> for Multipart {
    type Error = Box<dyn std::error::Error>;

    fn try_from(source: tmq::Multipart) -> Result<Self> {
        let mut v = vec![];
        for msg in source {
            v.push(msg.try_into()?);
        }
        Ok(Multipart(v))
    }
}

impl Deref for Multipart {
    type Target = [Message];

    fn deref(&self) -> &[Message] {
        &self.0
    }
}

/// Represents an error that may occur during a message reception.
#[derive(Debug)]
pub(crate) enum RecvError {
    /// A timeout occurred waiting for a message to arrive.
    Timeout,
    /// Some other kind of error occurred waiting for or receiving a
    /// message.
    Other(Error),
}

impl std::error::Error for RecvError {}

impl Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RecvError::Timeout => write!(f, "timeout"),
            RecvError::Other(e) => write!(f, "other({})", e),
        }
    }
}

/// Represents the type of socket to build with [`SocketBuilder`].
pub(crate) enum SocketType {
    /// The REQ socket.
    Req,
    // The REP socket.
    // Rep,
}

/// Configures and builds a ZeroMQ socket.
///
/// You can configure the kind of socket that you need with a
/// `SocketBuilder`.  The socket produced can be unidirectional or
/// bidirectional.  This depends on the kind of ZeroMQ socket you
/// configure.  You can configure your socket by calling one or more
/// modifier methods provided.  Once the configuration is complete,
/// invoking the [`connect`](#method.connect) method establishes a
/// connection.
///
/// Some modifiers are mandatory, such as
/// [`endpoint`](#method.endpoint).  Calling
/// [`connect`](#method.connect) before mandatory modifiers will fail.
///
/// # Example
///
/// Create a `SocketBuilder` to produce a REQ socket:
///
/// ```rust
/// // Initialise a ZMQ context.
/// use crate::keyword::kw;
/// use crate::socket::SocketBuilder;
///
/// let socket = SocketBuilder::new(context)
///     .endpoint("tcp://127.0.0.1:8888")
///     .timeout(200)
///     .linger(false)
///     .req()
///     .connect()?
///
/// let socket = socket.send_keyword(kw!["hello"]).await?;
/// let (response, socket) = socket.recv_string().await?;
/// println!("{}", response);
/// // Send more message with the returned socket.
/// ```
pub(crate) struct SocketBuilder {
    context: Context,
    endpoint: String,
    timeout: Option<u64>,
    linger: Option<bool>,
    socket_type: SocketType,
}

impl SocketBuilder {
    /// Creates a new `SocketBuilder` with the specified ZeroMQ
    /// context.
    pub(crate) fn new(context: Context) -> Self {
        SocketBuilder {
            context,
            endpoint: Default::default(),
            timeout: None,
            linger: None,
            socket_type: SocketType::Req,
        }
    }

    /// Sets the endpoint for the socket.
    pub(crate) fn endpoint(mut self, endpoint: &str) -> Self {
        self.endpoint = endpoint.to_owned();
        self
    }

    /// Sets the timeout value for socket operations.
    pub(crate) fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the linger option for the socket.
    pub(crate) fn linger(mut self, linger: bool) -> Self {
        self.linger = Some(linger);
        self
    }

    /// Sets the socket type to REQ (request).
    pub(crate) fn req(mut self) -> Self {
        self.socket_type = SocketType::Req;
        self
    }

    /// Connects to the configured endpoint and returns a
    /// `SocketSender` for sending messages.
    pub(crate) fn connect(self) -> Result<SocketSender> {
        use SocketType::*;
        let mut builder = match self.socket_type {
            Req => tmq::request(&self.context),
            // _ => unimplemented!(),
        };
        if let Some(linger) = self.linger {
            builder = builder.set_linger(if linger { 1 } else { 0 });
        }
        let socket = builder.connect(&self.endpoint)?;
        Ok(SocketSender {
            socket,
            timeout: self.timeout,
        })
    }
}

/// Represents a ZeroMQ socket for sending a message.
///
/// `SocketSender` can send messages to the target, but not receive.
/// This may be because of the kind of socket this is or the phase of
/// communication it is in.  For example, REQ can't receive any
/// messages before first sending a request.  In comparison, REP can't
/// send any messages before receiving a request.  If a `send_*`
/// method returns a `SocketReceiver`, you can use this to wait for
/// and receive the response.
///
/// # Examples
///
/// Send and receive messages using REQ socket:
///
/// ```rust
/// use crate::keyword::kw;
///
/// // Initialise a ZMQ context.
/// let socket = // build a REQ socket with SocketBuilder.
///
/// let socket = socket.send_keyword(kw!["hello"]).await?;
/// let (response, socket) = socket.recv_string().await?;
/// println!("{}", response);
/// // Send more message with the returned socket.
/// ```
pub(crate) struct SocketSender {
    socket: RequestSender,
    timeout: Option<u64>,
}

impl SocketSender {
    /// Sends a keyword.  Consumes the socket, but produces a new
    /// socket for waiting for and receiving the response.
    ///
    /// # Arguments
    ///
    /// * `keyword` - The keyword to send.
    ///
    /// # Returns
    ///
    /// Returns [`Ok`] with a [`SocketReceiver`] for receiving the
    /// response if successful.  Otherwise returns [`Err`] with the
    /// error object.
    ///
    /// # Examples
    ///
    /// Send a keyword and wait for the response:
    /// ```rust
    /// use crate::keyword::kw;
    /// let socket = socket.send_keyword(kw!["hello"]).await?;
    /// println!("{}", socket.recv_string().await?);
    /// ```
    pub(crate) async fn send_keyword(self, keyword: Keyword) -> Result<SocketReceiver> {
        Ok(SocketReceiver {
            socket: self.socket.send(vec![keyword.name()].into()).await?,
            timeout: self.timeout,
        })
    }

    /// Sends a sequence of keywords.  Consumes the socket, but
    /// produces a new socket for waiting for and receiving the
    /// response.
    ///
    /// # Arguments
    ///
    /// * `keywords` - The sequence of keywords to send.
    ///
    /// # Returns
    ///
    /// Returns [`Ok`] with a [`SocketReceiver`] for receiving the
    /// response if successful.  Otherwise returns [`Err`] with the
    /// error object.
    ///
    /// # Examples
    ///
    /// Send a sequence of keywords and wait for the response:
    /// ```rust
    /// use crate::keyword::kw;
    /// let socket = socket.send_keywords(&[kw!["hello"], kw!["world"]]).await?;
    /// println!("{}", socket.recv_string().await?);
    /// ```
    pub(crate) async fn send_keywords(self, keywords: &[Keyword]) -> Result<SocketReceiver> {
        let socket = self
            .socket
            .send(
                keywords
                    .iter()
                    .map(|kw| kw.name().to_owned().into_bytes())
                    .collect::<Vec<_>>()
                    .into(),
            )
            .await?;
        Ok(SocketReceiver {
            socket,
            timeout: self.timeout,
        })
    }
}

/// Represents a ZeroMQ socket for receiving a message.
///
/// `SocketReceiver` can receive messages from the target, but not
/// send. This may be because of the kind of socket this is or the
/// phase of communication it is in. For example, REQ can’t receive
/// any messages before first sending a request. In comparison, REP
/// can’t send any messages before receiving a request. If a recv_*
/// method returns a [`SocketSender`], you can use this to return a
/// message to the sender.
///
///
/// # Examples
///
/// Receive and return messages using REP socket:
///
/// ```rust
/// use crate::keyword::kw;
///
/// // Initialise a ZMQ context.
/// let socket = // build a REP socket with SocketBuilder.
///
/// let (message, socket) = socket.recv_string().await?;
/// println!("Received message: {}", message);
/// let socket = socket.send_keyword(kw!["ok"]).await?;
/// // Receive more message with the returned socket.
/// ```
pub(crate) struct SocketReceiver {
    socket: RequestReceiver,
    timeout: Option<u64>,
}

impl SocketReceiver {
    /// Receives a message as a string.  Consumes the socket, but
    /// produces a new socket for sending a response.
    ///
    /// # Returns
    ///
    /// Returns [`Ok`] with a tuple if successful.  The first element
    /// of the tuple is the message received as a string.  The second
    /// is a [`SocketSender`] for sending the response. On failure,
    /// returns [`Err`] with the error object.
    ///
    /// # Examples
    ///
    /// Receive a string and return the response:
    /// ```rust
    /// use crate::keyword::kw;
    /// let socket = // Prepare an REP socket.
    /// let (message, socket) = socket.recv_string().await?;
    /// println!("Message received: {}", message);
    /// let socket = socket.send_keyword(kw!["ok"]).await?;
    /// // Use socket to receive further message.
    /// ```
    pub(crate) async fn recv_string(
        self,
    ) -> std::result::Result<(String, SocketSender), RecvError> {
        let timeout = if let Some(timeout) = self.timeout {
            timeout
        } else {
            DEFAULT_SOCKET_TIMEOUT
        };

        match tokio::time::timeout(Duration::from_millis(timeout), self.socket.recv()).await {
            Ok(result) => result
                .map(|(multipart, sender)| {
                    (
                        multipart[0].as_str().unwrap().to_owned(),
                        SocketSender {
                            socket: sender,
                            timeout: self.timeout,
                        },
                    )
                })
                .map_err(|err| RecvError::Other(Box::new(err))),
            Err(_elapsed) => Err(RecvError::Timeout),
        }
    }

    /// Receives a multipart message.  Consumes the socket, but
    /// produces a new socket for sending a response.
    ///
    /// # Returns
    ///
    /// Returns [`Ok`] with a tuple if successful.  The first element
    /// of the tuple is the multipart message received.  The second is
    /// a [`SocketSender`] for sending the response. On failure,
    /// returns [`Err`] with the error object.
    ///
    /// # Examples
    ///
    /// Receive a multipart message and return the response:
    /// ```rust
    /// use crate::keyword::kw;
    /// let socket = // Prepare an REP socket.
    /// let (multipart, socket) = socket.recv_multipart().await?;
    /// println!("Multipart message received: {}", multipart);
    /// let socket = socket.send_keyword(kw!["ok"]).await?;
    /// // Use socket to receive further message.
    /// ```
    pub(crate) async fn recv_multipart(
        self,
    ) -> std::result::Result<(Multipart, SocketSender), RecvError> {
        let timeout = if let Some(timeout) = self.timeout {
            timeout
        } else {
            DEFAULT_SOCKET_TIMEOUT
        };

        match tokio::time::timeout(Duration::from_millis(timeout), self.socket.recv()).await {
            Ok(result) => {
                let (multipart, sender) = result.map_err(|err| RecvError::Other(Box::new(err)))?;
                Ok((
                    multipart.try_into().map_err(RecvError::Other)?,
                    SocketSender {
                        socket: sender,
                        timeout: self.timeout,
                    },
                ))
            }
            Err(_elapsed) => Err(RecvError::Timeout),
        }
    }
}
