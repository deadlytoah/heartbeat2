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
use crate::error::{illegal_state_error, ErrorType};
use crate::event::EventType;
use crate::logger::{LocalLogger, LogLevel};
use crate::result::Result;
use crate::signal::Signal;
use nix::unistd::Pid;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

/// Enumerates the possible statuses of the process managed by the
/// `ProcessManager`.
///
/// The `Status` enum represents the different states that the managed
/// process can be in at any given point in time.  It is used to track
/// and communicate the current status of the managed process.
///
/// # Examples
///
/// ```rust
/// use crate::process::Status;
///
/// let status = Status::Running;
/// match status {
///     Status::Ready => println!("Process is ready to start."),
///     Status::Running => println!("Process is currently running."),
///     Status::Terminated => println!("Process has terminated."),
///     Status::Killed => println!("Process has been killed."),
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub(crate) enum Status {
    /// Indicates that the process is ready to start or restart.
    Ready,
    /// Indicates that the process is currently running.
    Running,
    /// Indicates that the process has terminated normally.
    Terminated,
    /// Indicates that the process has been forcibly killed.
    Killed,
}

enum Action {
    RaiseSignal(Signal),
    Kill,
}

/// Enumerates the possible outcomes of a running process.
///
/// The `RunProcess` enum represents the different states or results
/// that can occur when running a process.  It is typically used by
/// the `RestartManager` to determine whether a process should be
/// restarted based on its completion status.
///
/// # Examples
///
/// ```rust
/// use crate::process::RunProcess;
///
/// fn handle_process_completion(result: RunProcess) {
///     match result {
///         RunProcess::Complete => println!("Process completed successfully."),
///         RunProcess::Abort => println!("Process aborted or encountered an error."),
///     }
/// }
/// ```
pub(crate) enum RunProcess {
    /// Indicates that the process has completed successfully.
    Complete,
    /// Indicates that the process has aborted or encountered an
    /// error.
    Abort,
}

/// Manages the execution and status of a process.
///
/// `ProcessManager` manages the execution and status of a process.
/// It starts, watches and controls the process.  It raises an event
/// when the status of the process changes.
/// [`EventHandler`](../event/struct.EventHandler.html) listens to
/// events like this and handles it according to the specification.
/// Raising status events is one of the central roles `ProcessManager`
/// plays in `Heartbeat2`.  `ProcessManager` uses a oneshot channel to
/// allow manipulation of the process status.  It raises events to
/// [`EventHandler`](../event/struct.EventHandler.html) via an MPSC
/// channel.  The orchestrated use of channels helps the state of
/// `ProcessManager` to stay consistent.
///
/// The specification specifies what various components of
/// `Heartbeat2` can do.  You can find it in spec/heartbeat.pdf in the
/// source repository.
///
/// # Examples
///
/// Creating a new `ProcessManager` and running a process:
///
/// ```rust
/// use crate::{ProcessManager, RunProcess};
///
/// async fn run_process_manager() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a process manager with event queue, configuration, and logger
///     let event_queue: mpsc::Sender<EventType> = // Event queue setup
///     let config: Rc<Config> = // Configuration setup
///     let logger: Rc<LocalLogger> = // Logger setup
///     let process_manager = ProcessManager::new(event_queue, config, logger);
///
///     // Run the process
///     let result = process_manager.run_process().await?;
///
///     // Handle the process outcome
///     match result {
///         RunProcess::Complete => println!("Process completed successfully."),
///         RunProcess::Abort => println!("Process aborted or encountered an error."),
///     }
///
///     Ok(())
/// }
/// ```
pub(crate) struct ProcessManager {
    status: Cell<Status>,
    agent: RefCell<Option<oneshot::Sender<Action>>>,
    event_queue: mpsc::Sender<EventType>,
    config: Rc<Config>,
    logger: Rc<LocalLogger>,
}

impl ProcessManager {
    /// Creates a new `ProcessManager` instance.
    ///
    /// # Arguments
    ///
    /// * `event_queue` - A sender channel for sending event types.
    /// * `config` - A shared reference to the configuration.
    /// * `logger` - A shared reference to the logger.
    ///
    /// # Returns
    ///
    /// A new `ProcessManager` instance.
    pub(crate) fn new(
        event_queue: mpsc::Sender<EventType>,
        config: Rc<Config>,
        logger: Rc<LocalLogger>,
    ) -> Self {
        ProcessManager {
            status: Cell::new(Status::Ready),
            agent: RefCell::new(None),
            event_queue,
            config,
            logger,
        }
    }

    /// Executes a process and returns its completion status.
    ///
    /// # Returns
    ///
    /// A `Result` containing a [`RunProcess`](enum.RunProcess.html)
    /// enum indicating the completion status of the process.
    ///
    /// # Errors
    ///
    /// `run_process()` returns an error if the `ProcessManager` is
    /// not in a ready state.  You can call [`reset()`](#method.reset)
    /// to prevent or recover from this error.
    pub(crate) async fn run_process(&self) -> Result<RunProcess> {
        let config_section = self.config.section(section::HEARTBEAT)?;
        let mut command = config_section.string_list(key::COMMAND)?;
        let exec: String = command.drain(0..1).collect();
        let args = command;
        let wd = config_section.string(key::WORKING_DIRECTORY)?;
        if self.is_ready() {
            self.logger.log(LogLevel::Info, "start process");
            self.set_status(Status::Running);
            let mut child = Command::new(exec).args(args).current_dir(wd).spawn()?;
            let (send_action, recv_action) = oneshot::channel::<Action>();
            self.agent.borrow_mut().replace(send_action);
            tokio::select! {
                exit_status = child.wait() => if exit_status?.success() {
                    self.raise_process_event_complete().await?;
                    Ok(RunProcess::Complete)
                } else {
                    self.raise_process_event_abort().await?;
                    Ok(RunProcess::Abort)
                },
                operation = recv_action => {
                    match operation? {
                        Action::RaiseSignal(signal) => {
                            if let Some(id) = child.id() {
                                nix::sys::signal::kill(Pid::from_raw(id.try_into()?), Some(signal.into()))?;
                            } else {
                                self.logger.log(LogLevel::Warning, &format!("unable to raise signal [{:?}] as child process already exited", signal))
                            }
                            Ok(RunProcess::Complete)
                        }
                        Action::Kill => {
                            child.start_kill()?;
                            let _ = child.wait().await;
                            Ok(RunProcess::Abort)
                        }
                    }
                }
            }
        } else {
            Err(illegal_state_error(&format!("{:?}", self.status())))
        }
    }

    /// Reset the state of the `ProcessManager`.
    ///
    /// This method resets the state of the `ProcessManager` to
    /// `Ready` if it is currently in the `Killed` state.  If the
    /// `ProcessManager` is not in the `Killed` state, an error is
    /// returned.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success of the operation. If the
    /// reset is successful, `Ok(())` is returned.
    ///
    /// # Errors
    ///
    /// An error is returned if the `ProcessManager` is not in the
    /// `Killed` state.
    pub(crate) fn reset(&self) -> Result<()> {
        self.logger.log(LogLevel::Trace, "ProcessManager::reset()");
        if self.is_killed() {
            self.set_status(Status::Ready);
            Ok(())
        } else {
            Err(illegal_state_error(&format!("{:?}", self.status())))
        }
    }

    /// Kills the managed process.
    ///
    /// Sets the status of the process to `Killed` and sends the kill
    /// message to the process action channel.  The process action
    /// channel is useful for performing a specific action to the
    /// process.  It does this in a synchronous way.  In operating
    /// systems like Unix, killing a process is sending the process a
    /// KILL signal.  But `kill_process` is a separate function
    /// because it uses a platform independent function.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success of the operation. If the
    /// process is successfully killed, `Ok(())` is returned.
    ///
    /// # Errors
    ///
    /// An error is returned if there is no running process or if the
    /// action sending fails.
    pub(crate) fn kill_process(&self) -> std::result::Result<(), ErrorType> {
        self.logger
            .log(LogLevel::Trace, "ProcessManager::kill_process()");
        self.set_status(Status::Killed);
        self.agent
            .borrow_mut()
            .take()
            .ok_or(ErrorType::NoRunningProcess)?
            .send(Action::Kill)
            .map_err(|_| ErrorType::NoRunningProcess)?;
        Ok(())
    }

    /// Signals the managed process.
    ///
    /// Sets the status of the process to `Terminated`.  Then sends
    /// the `RaiseSignal` message to the process action channel.  The
    /// process action channel is useful for performing a specific
    /// action to the process.  It does this in a synchronous way.  In
    /// operating systems like Unix, killing a process is sending the
    /// process a KILL signal.  But `raise_signal` is a separate
    /// function from `kill_process`.  This is because unlike
    /// `kill_process`, it uses a function specific to Unix.
    ///
    /// # Arguments
    ///
    /// * `signal` - The signal to be raised.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success of the operation. If the
    /// signal is successfully raised, `Ok(())` is returned.
    ///
    /// # Errors
    ///
    /// An error is returned if there is no running process or if the
    /// action sending fails.
    pub(crate) fn raise_signal(&self, signal: Signal) -> std::result::Result<(), ErrorType> {
        self.logger.log(
            LogLevel::Trace,
            &format!("ProcessManager::raise_signal({:?})", signal),
        );
        self.set_status(Status::Terminated);
        self.agent
            .borrow_mut()
            .take()
            .ok_or(ErrorType::NoRunningProcess)?
            .send(Action::RaiseSignal(signal))
            .map_err(|_| ErrorType::NoRunningProcess)?;
        Ok(())
    }

    /// Check if the `ProcessManager` is in the `Killed` state.
    ///
    /// # Returns
    ///
    /// `true` if the `ProcessManager` is in the `Killed` state,
    /// `false` otherwise.
    pub(crate) fn is_killed(&self) -> bool {
        matches!(self.status(), Status::Killed)
    }

    /// Set the status of the `ProcessManager` to `Killed`.
    pub(crate) fn set_killed(&self) {
        self.set_status(Status::Killed);
    }

    /// Check if the `ProcessManager` is in the `Terminated` state.
    ///
    /// # Returns
    ///
    /// `true` if the `ProcessManager` is in the `Terminated` state,
    /// `false` otherwise.
    pub(crate) fn is_terminated(&self) -> bool {
        matches!(self.status(), Status::Terminated)
    }

    /// Set the status of the `ProcessManager` to `Terminated`.
    pub(crate) fn set_terminated(&self) {
        self.set_status(Status::Terminated);
    }

    /// Raises an event indicating that the process has completed.
    async fn raise_process_event_complete(&self) -> Result<()> {
        self.logger.log(LogLevel::Info, "normal process exit");
        self.event_queue.send(EventType::Complete).await?;
        Ok(())
    }

    /// Raises an event indicating that the process has aborted.
    async fn raise_process_event_abort(&self) -> Result<()> {
        self.logger.log(LogLevel::Error, "abnormal process exit");
        self.event_queue.send(EventType::Aborted).await?;
        Ok(())
    }

    fn set_status(&self, status: Status) {
        self.status.set(status);
    }

    fn status(&self) -> Status {
        self.status.get()
    }

    fn is_ready(&self) -> bool {
        matches!(self.status(), Status::Ready)
    }
}
