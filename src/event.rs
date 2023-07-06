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

use crate::heartbeat::Heartbeat;
use crate::logger::{LocalLogger, LogLevel};
use crate::result::Result;
use crate::signal::{Signal, SignalHandler};
use crate::ProcessManager;
use std::rc::Rc;
use tokio::sync::mpsc::{self, error::TryRecvError};

/// EventType describes the type of event that affects the health or
/// lifecycle of the monitored process.
#[derive(Debug)]
pub(crate) enum EventType {
    /// Event indicating a heartbeat timeout.
    Timeout,
    /// Event indicating a process abortion.
    Aborted,
    /// Event indicating a process completion.
    Complete,
    /// Event indicating a process signal with the associated signal
    /// type.
    Signalled(Signal),
}

/// Receives events from various components of the heartbeat2
/// application and handles them.
///
/// The `EventHandler` struct is responsible for receiving events from
/// different components of the heartbeat2 application. These events
/// can include heartbeat timeouts, process abortions, process
/// completions, and process signals. The `EventHandler` logs the
/// received events and makes decisions based on them, such as whether
/// to restart the process.
///
/// # Example
///
/// ```rust
/// use crate::{EventHandler, EventType, ProcessManager, Heartbeat, SignalHandler, LocalLogger};
/// use std::sync::mpsc;
/// use std::rc::Rc;
///
/// // Create necessary components and event receiver
/// let (event_sender, event_receiver) = mpsc::channel();
/// let process_manager = Rc::new(ProcessManager::new());
/// let heartbeat = Rc::new(Heartbeat::new());
/// let signal_handler = Rc::new(SignalHandler::new());
/// let logger = Rc::new(LocalLogger::new());
///
/// // Create and initialize the event handler
/// let event_handler = EventHandler::new(event_receiver,
///                                       process_manager.clone(),
///                                       heartbeat.clone(),
///                                       signal_handler.clone(),
///                                       logger.clone());
///
/// // Spawn a thread or start an event loop to handle events
/// // ...
/// ```
pub(crate) struct EventHandler {
    event_receiver: mpsc::Receiver<EventType>,
    process_manager: Rc<ProcessManager>,
    heartbeat: Rc<Heartbeat>,
    signal_handler: Rc<SignalHandler>,
    logger: Rc<LocalLogger>,
}

impl EventHandler {
    /// Creates a new `EventHandler` instance.
    ///
    /// # Arguments
    ///
    /// * `event_receiver` - The receiver channel to receive
    ///                      `EventType` events.
    /// * `process_manager` - The shared `ProcessManager` instance.
    /// * `heartbeat` - The shared `Heartbeat` instance.
    /// * `signal_handler` - The shared `SignalHandler` instance.
    /// * `logger` - The shared `LocalLogger` instance.
    ///
    /// # Returns
    ///
    /// Returns a new `EventHandler` object.
    pub(crate) fn new(
        event_receiver: mpsc::Receiver<EventType>,
        process_manager: Rc<ProcessManager>,
        heartbeat: Rc<Heartbeat>,
        signal_handler: Rc<SignalHandler>,
        logger: Rc<LocalLogger>,
    ) -> Self {
        EventHandler {
            event_receiver,
            process_manager,
            heartbeat,
            signal_handler,
            logger,
        }
    }

    /// Runs the event handling loop for the `EventHandler`.
    ///
    /// The `run` method runs the event handling loop for the
    /// `EventHandler`. It continuously listens for events from the
    /// event receiver and processes them until either the process is
    /// terminated or killed. Upon receiving an event, the method logs
    /// and handles the event according to the specification. The
    /// specification details the event handling logic and can be
    /// found in spec/heartbeat.pdf in the source repository.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating the success or failure of the
    /// event handling loop. It returns Ok when it ends due to a
    /// process kill or termination. Event handling logic can fail for
    /// various reasons. In this case, it returns an error with the
    /// corresponding error message.
    ///
    /// # Panics
    ///
    /// Panics when all its event sources close the sending ends of
    /// the event channel. `EventHandler` holds the only receiving end
    /// of the event channel. It expects itself to be the last task to
    /// close the communications link. This is because we don't want
    /// the risk of losing an important event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::{EventHandler, EventType, ProcessManager, LocalLogger};
    /// use std::sync::mpsc;
    /// use std::rc::Rc;
    ///
    /// // Create necessary components and event receiver
    /// let (event_sender, event_receiver) = mpsc::channel();
    /// let process_manager = Rc::new(ProcessManager::new());
    /// let heartbeat = Rc::new(Heartbeat::new());
    /// let signal_handler = Rc::new(SignalHandler::new());
    /// let logger = Rc::new(LocalLogger::new());
    ///
    /// // Create and initialize the event handler
    /// let mut event_handler = EventHandler::new(event_receiver,
    ///                                           process_manager.clone(),
    ///                                           heartbeat.clone(),
    ///                                           signal_handler.clone(),
    ///                                           logger.clone());
    ///
    /// // Start the event handling loop
    /// if let Err(err) = event_handler.run() {
    ///     eprintln!("Error occurred during event handling: {:?}", err);
    /// }
    /// ```
    pub(crate) async fn run(&mut self) -> Result<()> {
        while !self.process_manager.is_terminated() && !self.process_manager.is_killed() {
            if let Some(event_type) = self.event_receiver.recv().await {
                self.logger
                    .log(LogLevel::Debug, &format!("[{:?}] event raised", event_type));
                match event_type {
                    EventType::Timeout => self.consume_timeout_event()?,
                    EventType::Aborted => self.consume_aborted_event()?,
                    EventType::Complete => self.consume_complete_event()?,
                    EventType::Signalled(sig) => self.consume_signaled_event(sig)?,
                }
            } else {
                // Queue is closed, and no more messages are in the
                // queue.
                panic!("event queue closed");
            }
        }
        Ok(())
    }

    /// Resets the state of the `EventHandler`.
    ///
    /// The `reset` method resets the state of the `EventHandler`. It
    /// clears the event queue and performs any necessary cleanup or
    /// initialization steps to prepare the `EventHandler` for
    /// handling new events.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::{EventHandler, LogLevel, LocalLogger};
    /// use std::sync::mpsc;
    ///
    /// // Create necessary components and event receiver
    /// let (event_sender, event_receiver) = mpsc::channel();
    /// let process_manager = Rc::new(ProcessManager::new());
    /// let heartbeat = Rc::new(Heartbeat::new());
    /// let signal_handler = Rc::new(SignalHandler::new());
    /// let logger = Rc::new(LocalLogger::new());
    ///
    /// // Create and initialize the event handler
    /// let mut event_handler = EventHandler::new(event_receiver,
    ///                                           process_manager.clone(),
    ///                                           heartbeat.clone(),
    ///                                           signal_handler.clone(),
    ///                                           logger.clone());
    ///
    /// // Reset the event handler
    /// event_handler.reset();
    /// ```
    pub(crate) fn reset(&mut self) {
        self.logger.log(LogLevel::Trace, "EventHandler::reset()");
        self.clear_queue();
    }

    fn consume_timeout_event(&self) -> Result<()> {
        self.process_manager.kill_process()?;
        self.signal_handler.close();
        Ok(())
    }

    fn consume_aborted_event(&self) -> Result<()> {
        self.logger
            .log(LogLevel::Trace, "EventHandler::consume_aborted_event()");
        self.process_manager.set_killed();
        self.heartbeat.stop()?;
        self.signal_handler.close();
        Ok(())
    }

    fn consume_complete_event(&self) -> Result<()> {
        self.logger
            .log(LogLevel::Trace, "EventHandler::consume_complete_event()");
        self.process_manager.set_terminated();
        self.heartbeat.stop()?;
        self.signal_handler.close();
        Ok(())
    }

    fn consume_signaled_event(&self, signal: Signal) -> Result<()> {
        self.logger.log(
            LogLevel::Trace,
            &format!("EventHandler::consume_signaled_event({:#?})", signal),
        );
        self.process_manager.raise_signal(signal)?;
        self.heartbeat.stop()?;
        self.signal_handler.close();
        Ok(())
    }

    fn clear_queue(&mut self) {
        loop {
            match self.event_receiver.try_recv() {
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("event queue closed"),
                Ok(_) => (),
            }
        }
    }
}
