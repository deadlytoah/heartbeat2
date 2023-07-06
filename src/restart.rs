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
use crate::logger::{LocalLogger, LogLevel};
use crate::result::Result;
use std::rc::Rc;

/// Manages the restart behavior of a process.
///
/// Tracks the process restarts, and determines whether to restart the
/// process or give up.  It follows the Heartbeat specification to
/// make the decision to restart a process.  The Heartbeat
/// specification is available in src/spec/heartbeat.pdf.  It
/// documents what various components of `Heartbeat2` can do.
/// `RestartManager` refers to the configuration and the restart
/// history to make the decision.
///
/// # Configuration
///
/// The configuration settings `RestartManager` uses to determine the
/// process restart are as follows:
///
/// * RETRY-INTERVAL: `RestartManager` determines whether the process
/// restarts too many times in a period.  `Heartbeat2` gives up
/// restarting the process in this case.  This integer parameter
/// configures the period in seconds.
/// * MAX-RETRIES: Configures the number of restarts before giving up.
///
/// # Examples
///
/// Create a `RestartManager`:
///
/// ```rust
/// use crate::config::Config;
/// use crate::logger::{LocalLogger, LogLevel::Info};
/// use crate::restart::RestartManager;
///
/// // Create a restart manager with configuration and logger
/// let config: Rc<Config> = // Configuration setup
/// let logger: Rc<LocalLogger> = // Logger setup
/// let mut restart_manager = RestartManager::new(config, logger);
/// ```
///
/// Add a new restart in the history:
///
/// ```rust
/// restart_manager.add_process_abort()?;
/// ```
///
/// Determine whether to restart the process:
///
/// ```rust
/// if restart_manager.should_process_restart()? {
///     logger.log(INFO, "Restarting process.");
///     restart_process().await?;
/// } else {
///     logger.log(INFO, "Giving up.");
/// }
/// ```
pub(crate) struct RestartManager {
    history: Vec<i64>,
    config: Rc<Config>,
    logger: Rc<LocalLogger>,
}

impl RestartManager {
    /// Create a new `RestartManager` instance.
    ///
    /// # Arguments
    ///
    /// * `config` - The shared configuration for the restart manager.
    /// * `logger` - The logger used for logging restart events.
    ///
    /// # Returns
    ///
    /// A new `RestartManager` instance.
    pub(crate) fn new(config: Rc<Config>, logger: Rc<LocalLogger>) -> RestartManager {
        RestartManager {
            history: Default::default(),
            config,
            logger,
        }
    }

    /// Determines whether to restart the process.
    ///
    /// Decides whether `Heartbeat2` should restart the managed
    /// process.  Bases this decision on the restart history and the
    /// configured restart policies.  The specification found under
    /// spec/heartbeat.pdf specifies the behaviour of this method.
    /// Restarting means the managed process has failed, but we give
    /// it another chance to succeed.  Not restarting means deciding
    /// that the managed process’ failure is persistent.  An engineer
    /// needs to log in and take a closer look in this case.
    ///
    /// The method expects the caller to restart the managed process.
    /// `RestartManager` is unable to restart the process.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` to direct that the process must restart.
    /// Returns `Ok(false)` if `Heartbeat2` should give up on the
    /// process and exit.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue accessing the
    /// configuration.
    pub(crate) fn should_process_restart(&mut self) -> Result<bool> {
        Ok(!self.too_many_retries()?)
    }

    /// Records a restart in the restart history.
    ///
    /// Adds the current timestamp in the restart history.
    /// `Heartbeat2` uses the restart history to decide if the process
    /// is restarting too often.  Prunes the restart history to
    /// prevent it from becoming too large.  Large restart history
    /// wastes the memory and adds latency.
    ///
    /// # Returns
    ///
    /// Indicates success or failure.
    ///
    /// # Errors
    ///
    /// Returns an error if it fails to read from the configuration.
    ///
    /// # Note
    ///
    /// I call it the restart history because a process abort usually
    /// leads to a process restart.  Otherwise, `Heartbeat2`
    /// terminates.  So the restart history equates to the record of
    /// process aborts in this case.
    pub(crate) fn add_process_abort(&mut self) -> Result<()> {
        self.prune()?;
        self.history.push(chrono::Utc::now().timestamp());
        self.logger.log(
            LogLevel::Debug,
            &format!("RestartManager: current history: {:?}", self.history),
        );
        Ok(())
    }

    fn too_many_retries(&self) -> Result<bool> {
        let section = self.config.section(section::HEARTBEAT)?;
        let retry_interval = section.integer(key::RETRY_INTERVAL)?;
        let max_retries = section.integer(key::MAX_RETRIES)?;
        let now = chrono::Utc::now().timestamp();
        let retries: i64 = self
            .history
            .iter()
            .filter(|&&item| item >= now - retry_interval)
            .count()
            .try_into()?;
        Ok(retries >= max_retries)
    }

    fn prune(&mut self) -> Result<()> {
        let max_retries = self
            .config
            .section(section::HEARTBEAT)?
            .integer(key::MAX_RETRIES)?
            .try_into()?;
        while self.history.len() >= max_retries {
            self.history.remove(0);
        }
        Ok(())
    }
}
