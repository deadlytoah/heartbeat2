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
use crate::error::{illegal_state_error, peer_channel_closed_error};
use crate::event::EventType;
use crate::kw;
use crate::logger::{LocalLogger, LogLevel};
use crate::result::Result;
use crate::socket::{RecvError, SocketBuilder};
use crate::Sup;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tmq::{self, Context};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration};

/// Represents the status of the Heartbeat at a given point in time.
///
/// The `Status` enum describes the possible statuses of the Heartbeat
/// component at any given point in time. It is used to indicate the
/// current state of the Heartbeat, such as whether it is ready to
/// send heartbeats (`Ready`), actively waiting for a response
/// (`Req`), or has timed out without receiving a response
/// (`Timeout`).
///
/// The specification details the possible statuses of the Heartbeat
/// and their precise meanings.  You can find the specification in the
/// project repository under spec/heartbeat.pdf.
///
/// # Example
///
/// ```rust
/// use crate::Status;
///
/// let status = Status::Ready;
///
/// match status {
///     Status::Ready => println!("Heartbeat is ready."),
///     Status::Req => println!("Heartbeat is waiting for a response."),
///     Status::Timeout => println!("Heartbeat has timed out."),
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub(crate) enum Status {
    /// Indicates that the Heartbeat is ready to send heartbeats.
    Ready,
    /// Indicates that the Heartbeat is actively waiting for a
    /// response.
    Req,
    /// Indicates that the Heartbeat has timed out without receiving a
    /// response.
    Timeout,
}

enum TimerFuncResult {
    Continue,
    Break,
}

/// The Heartbeat component is responsible for sending regular
/// heartbeats and raising timeout events.
///
/// The `Heartbeat` struct represents the Heartbeat component in the
/// application. It is responsible for sending regular heartbeats to
/// the target application and raising timeout events if no response
/// is received within the configured time. The `Heartbeat` struct
/// contains various fields such as the ZeroMQ context, configuration,
/// the proxy object to the naming service (Sup), logger, status and
/// channels for quiting Heartbeat loop and event notifications.
pub(crate) struct Heartbeat {
    context: Context,
    config: Rc<Config>,
    sup: Rc<Sup>,
    logger: Rc<LocalLogger>,
    status: Cell<Status>,
    send_stop: RefCell<Option<oneshot::Sender<()>>>,
    send_event: mpsc::Sender<EventType>,
}

impl Heartbeat {
    /// Constructs a new `Heartbeat` instance.
    ///
    /// The `new` function creates a new `Heartbeat` instance with the
    /// specified parameters.  It takes a ZeroMQ context (`context`),
    /// a channel for sending event notifications (`send_event`), a
    /// shared reference to the configuration (`config`), a shared
    /// reference to the naming service (`sup`), and a shared
    /// reference to the logger (`logger`).
    ///
    /// # Arguments
    ///
    /// * `context` - The ZeroMQ context for the Heartbeat.
    /// * `send_event` - The channel for sending event notifications.
    /// * `config` - A shared reference to the configuration.
    /// * `sup` - A shared reference to the naming service.
    /// * `logger` - A shared reference to the logger.
    ///
    /// # Returns
    ///
    /// Returns a new `Heartbeat` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::{Context, EventType, Config, Sup, LocalLogger};
    ///
    /// let (send_event, recv_event) = mpsc::channel();
    /// let context = Context::new();
    /// let config = Rc::new(Config::new());
    /// let sup = Rc::new(Sup::new());
    /// let logger = Rc::new(LocalLogger::new());
    ///
    /// let heartbeat = Heartbeat::new(context, send_event, config, sup, logger);
    /// ```
    pub(crate) fn new(
        context: Context,
        send_event: mpsc::Sender<EventType>,
        config: Rc<Config>,
        sup: Rc<Sup>,
        logger: Rc<LocalLogger>,
    ) -> Self {
        Heartbeat {
            context,
            config,
            sup,
            logger,
            status: Cell::new(Status::Ready),
            send_stop: RefCell::new(None),
            send_event,
        }
    }

    /// Runs the heartbeat process.
    ///
    /// The `run` function starts the `Heartbeat` task, kicking off
    /// its timer loop.  The `Heartbeat` task must be in the correct
    /// status in order for it to run.  If it is not in the correct
    /// status, `run` returns the illegal status error.  If this is
    /// the case, `reset` function can put `Heartbeat` in the correct
    /// status for starting.  Once started, it sends a heartbeat
    /// message to the target application, and waits for a response.
    /// It raises Timeout event using the event channel sender if
    /// there is no response for a period.  `EventHandler` consumes
    /// the Timeout event to decide what to do with the target
    /// process.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating success (`Ok`) if the heartbeat
    /// process is executed successfully, or an error (`Err`) if the
    /// heartbeat is not in the "Ready" state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::{LogLevel, Result};
    ///
    /// async fn example_run() -> Result<()> {
    ///     let heartbeat = Heartbeat::new(/* parameters */);
    ///     heartbeat.run().await?;
    ///     Ok(())
    /// }
    /// ```
    pub(crate) async fn run(&self) -> Result<()> {
        if self.is_ready() {
            self.logger.log(LogLevel::Info, "start heartbeat");
            self.timer_loop().await?;
            Ok(())
        } else {
            Err(illegal_state_error(&format!("{:?}", self.status)))
        }
    }

    /// Stops the `Heartbeat` task.
    ///
    /// The `stop` function stops the `Heartbeat` task. It then
    /// attempts to send a stop signal to the internal timer loop. If
    /// the signaling is successful or the `Heartbeat` task is already
    /// stopped, it returns `Ok(())`. An error returned indicates
    /// there was a problem sending the stop signal to the timer
    /// loop. This would mean the receiving end of the stop channel
    /// closed the channel, which would be a logic error.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating success (`Ok`) if the heartbeat
    /// process is stopped successfully, or an error (`Err`) if an
    /// error occurs while sending the stop signal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::{LogLevel, Result};
    ///
    /// fn example_stop(heartbeat: &Heartbeat) -> Result<()> {
    ///     heartbeat.stop()?;
    ///     Ok(())
    /// }
    /// ```
    pub(crate) fn stop(&self) -> Result<()> {
        self.logger.log(LogLevel::Trace, "Heartbeat::stop()");
        match self
            .send_stop
            .borrow_mut()
            .take()
            .map(|send_stop| send_stop.send(()))
        {
            Some(Ok(_)) | None => Ok(()),
            Some(Err(_)) => Err(peer_channel_closed_error()),
        }
    }

    /// Resets the status of the `Heartbeat` task so that it can start
    /// again.
    pub(crate) fn reset(&self) {
        self.set_status(Status::Ready);
    }

    /// Returns whether the instance of the `Heartbeat` task is in a
    /// status where it can start.
    ///
    /// # Returns
    ///
    /// Returns true if the `Heartbeat` task can start, but false if
    /// starting it will cause an error.
    pub(crate) fn is_ready(&self) -> bool {
        matches!(self.status(), Status::Ready)
    }

    /// Returns the target service's endpoint by looking up
    /// :target-endpoint key.  If this key is missing, looks up
    /// :target-id key, and then uses SUP to resolve its value to an
    /// endpoint.  Returns the endpoint.  Having heartbeat2 check
    /// :target-endpoint setting first is meant to liberate it from a
    /// tight dependency on SUP.
    ///
    /// # Returns
    ///
    /// Returns the endpoint, or an error if something goes wrong
    /// reading the configuration or looking up the application ID
    /// with the naming service.
    async fn app_endpoint(&self) -> Result<String> {
        let heartbeat_section = self.config.section(section::HEARTBEAT)?;
        if heartbeat_section.has_key(key::TARGET_ENDPOINT) {
            let endpoint = heartbeat_section.target_endpoint()?;
            self.logger
                .log(LogLevel::Debug, &format!("endpoint: {}", endpoint));
            Ok(endpoint.to_owned())
        } else {
            let app_id = heartbeat_section.target_id()?;
            let endpoint = self.sup.sget(app_id).await?;
            self.logger.log(
                LogLevel::Debug,
                &format!("endpoint of app {}: {}", app_id, endpoint),
            );
            Ok(endpoint)
        }
    }

    async fn beat(&self) -> Result<Status> {
        let endpoint = self.app_endpoint().await?;
        let timeout = self
            .config
            .section(section::HEARTBEAT)?
            .heartbeat_timeout()?;
        let socket = SocketBuilder::new(self.context.clone())
            .endpoint(&endpoint)
            .timeout(timeout)
            .linger(false)
            .req()
            .connect()?;
        let recv_sock = socket.send_keyword(kw![heartbeat]).await?;
        self.set_status(Status::Req);
        match recv_sock.recv_string().await {
            Ok(_) => Ok(Status::Ready),
            Err(RecvError::Timeout) => Ok(Status::Timeout),
            Err(RecvError::Other(err)) => Err(err),
        }
    }

    async fn timer_func(&self) -> Result<TimerFuncResult> {
        self.logger.log(LogLevel::Trace, "timer_func");
        let new_status = self.beat().await?;
        self.set_status(new_status);
        match new_status {
            Status::Ready => Ok(TimerFuncResult::Continue),
            Status::Timeout => {
                self.logger.log(LogLevel::Error, "heartbeat timed out");
                self.send_event.send(EventType::Timeout).await?;
                Ok(TimerFuncResult::Break)
            }
            _ => Err(illegal_state_error(&format!("{:?}", new_status))),
        }
    }

    async fn timer_loop(&self) -> Result<()> {
        use TimerFuncResult::*;
        let interval = Duration::from_secs(
            self.config
                .section(section::HEARTBEAT)?
                .integer(key::HEARTBEAT_INTERVAL)?
                .try_into()?,
        );

        loop {
            let (send_stop, recv_stop) = oneshot::channel();
            self.send_stop.replace(Some(send_stop));

            tokio::select! {
                _ = sleep(interval) => (),
                _ = recv_stop => break,
            }
            self.logger.log(LogLevel::Trace, "heartbeat wakes up");
            match self.timer_func().await? {
                Continue => self.logger.log(
                    LogLevel::Trace,
                    &format!("next heartbeat in {}s", interval.as_secs()),
                ),
                Break => break,
            }
        }
        Ok(())
    }

    fn status(&self) -> Status {
        self.status.get()
    }

    fn set_status(&self, status: Status) {
        self.status.set(status);
    }
}
