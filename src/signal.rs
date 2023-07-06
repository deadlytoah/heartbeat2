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

use crate::event::EventType;
use crate::logger::{LocalLogger, LogLevel};
use crate::result::Result;
use futures::stream::StreamExt;
use signal_hook::consts::signal::{SIGQUIT, SIGTERM};
use signal_hook_tokio::{Handle, Signals};
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc::Sender;

/// Represents UNIX signals that [`SignalHandler`] actions on.
///
/// The OS or the user can sometimes signal the `Heartbeat2` process.
/// [`SignalHandler`] actions on a subset of UNIX signals and raises
/// corresponding events.
/// [`EventHandler`](crate::event::EventHandler) receives these events
/// and have them handled.  `enum Signal` represents the subset of
/// UNIX signals that [`SignalHandler`] actions on.  [`SignalHandler`]
/// ignores the other signals.
///
/// Each member in this `enum` corresponds to a UNIX signal.  `enum
/// Signal` implements [`From<Signal>`].  This allows conversion of an
/// `enum Signal` to the corresponding UNIX signal.
///
/// How `Heartbeat2` handles incoming `SIGTERM` is different from
/// `SIGQUIT`.  `Heartbeat2` relays `SIGTERM` to the managed process
/// to cause a normal exit.  Both the managed process and `Heartbeat2`
/// will exit after `SIGTERM`.  But `SIGQUIT` causes only the
/// `Heartbeat2` process to exit.  The managed process will still be
/// running after `SIGQUIT`.
pub(crate) enum Signal {
    /// Indicates the `Heartbeat2` process has received a `SIGQUIT`.
    Quit,
    /// Indicates the `Heartbeat2` process has received a `SIGTERM`.
    Term,
}

impl From<Signal> for nix::sys::signal::Signal {
    fn from(source: Signal) -> Self {
        match source {
            Signal::Quit => Self::SIGQUIT,
            Signal::Term => Self::SIGTERM,
        }
    }
}

/// Forwards signal to [`EventHandler`](crate::event::EventHandler).
///
/// Actions on signal by raising an appropriate event to
/// [`EventHandler`](crate::event::EventHandler).  [`Signal`] defines
/// the subset of UNIX signals `SignalHandler` reacts to.
pub(crate) struct SignalHandler {
    event_sender: Sender<EventType>,
    signal_handle: RefCell<Option<Handle>>,
    logger: Rc<LocalLogger>,
}

impl SignalHandler {
    /// Creates a new `SignalHandler` with the specified event sender
    /// and logger.
    pub(crate) fn new(event_sender: Sender<EventType>, logger: Rc<LocalLogger>) -> Self {
        Self {
            event_sender,
            signal_handle: RefCell::new(None),
            logger,
        }
    }

    /// Runs the signal handling loop, waiting for signals and sending
    /// corresponding event types to the event sender.
    pub(crate) async fn run(&self) -> Result<()> {
        let mut signals = Signals::new(&[SIGQUIT, SIGTERM])?;
        let old_handle = self.signal_handle.replace(Some(signals.handle()));
        // NOTE: Close the old handle before calling run().
        debug_assert!(matches!(old_handle, None));
        while let Some(signal) = signals.next().await {
            match signal {
                SIGQUIT => {
                    self.event_sender
                        .send(EventType::Signalled(Signal::Quit))
                        .await?
                }
                SIGTERM => {
                    self.event_sender
                        .send(EventType::Signalled(Signal::Term))
                        .await?
                }
                _ => unreachable!("unhandled signal"),
            }
        }
        Ok(())
    }

    /// Closes the `SignalHandler`.
    ///
    /// Closing the `SignalHandler` means it will no longer forward
    /// signal to the [`EventHandler`](crate::event::EventHandler).
    pub(crate) fn close(&self) {
        self.logger.log(LogLevel::Trace, "SignalHandler::close()");
        let handle = self
            .signal_handle
            .borrow_mut()
            .take()
            .expect("signal handle missing");
        handle.close();
    }
}
