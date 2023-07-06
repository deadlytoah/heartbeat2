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

mod config;
mod error;
mod event;
mod expression;
mod heartbeat;
mod keyword;
pub mod logger;
mod plist;
mod process;
mod restart;
mod result;
mod signal;
mod socket;
mod sup;

use crate::config::{key, section};
use crate::event::EventHandler;
use crate::heartbeat::Heartbeat;
use crate::logger::{LocalLogger, LogLevel, LogLevel::Info};
use crate::process::{ProcessManager, RunProcess};
use crate::restart::RestartManager;
use crate::result::Result;
use crate::signal::SignalHandler;
use crate::sup::Sup;
use config::Config;
use std::rc::Rc;
use tmq::Context;
use tokio::sync::mpsc::channel;

/// The unique app identifier
static APP_ID: &str = "HEARTBEAT";

/// The path to the configuration file.
static DEFAULT_CONFIG_FILE_NAME: &str = "heartbeat.cfg";

/// The size of the event queue
static EVENT_QUEUE_SIZE: usize = 1;

async fn main_impl(config: Config, logger: Rc<LocalLogger>) -> Result<()> {
    let config = Rc::new(config);
    let context = Context::new();
    let sup = Rc::new(Sup::with_context(context.clone(), Rc::clone(&config)));
    logger.log(
        LogLevel::Info,
        &format!(
            "start heartbeat process (PID {}) for target [{}]",
            std::process::id(),
            config.section(section::HEARTBEAT)?.target_id()?
        ),
    );

    let (event_sender, event_receiver) = channel(EVENT_QUEUE_SIZE);
    let heartbeat = Rc::new(Heartbeat::new(
        context.clone(),
        event_sender.clone(),
        Rc::clone(&config),
        Rc::clone(&sup),
        Rc::clone(&logger),
    ));
    let signal_handler = Rc::new(SignalHandler::new(event_sender.clone(), Rc::clone(&logger)));
    let process_manager = Rc::new(ProcessManager::new(
        event_sender.clone(),
        Rc::clone(&config),
        Rc::clone(&logger),
    ));

    let mut event_handler = EventHandler::new(
        event_receiver,
        Rc::clone(&process_manager),
        Rc::clone(&heartbeat),
        Rc::clone(&signal_handler),
        Rc::clone(&logger),
    );

    let mut restart_manager = RestartManager::new(Rc::clone(&config), Rc::clone(&logger));

    loop {
        let (_, run_process, _, _) = tokio::try_join!(
            heartbeat.run(),
            process_manager.run_process(),
            signal_handler.run(),
            event_handler.run(),
        )?;
        match run_process {
            RunProcess::Abort => {
                restart_manager.add_process_abort()?;
                if restart_manager.should_process_restart()? {
                    logger.log(LogLevel::Info, "attempt to restart process");
                    process_manager.reset()?;
                    heartbeat.reset();
                    event_handler.reset();
                    // Drop through to the beginning of the loop.
                } else {
                    logger.log(LogLevel::Info, "giving up due to too many retries");
                    process_manager.set_terminated();
                    break;
                }
            }
            RunProcess::Complete => {
                break;
            }
        }
    }
    Ok(())
}

/// Checks if the provided `config` requires the "sup" service to
/// resolve a service name and produce an endpoint address for IPC.
/// If the `config` provides the endpoint of the target service,
/// the "sup" service isn't required.
///
/// # Arguments
///
/// * `config` - A reference to the `Config` struct containing the
/// configuration information.
///
/// # Returns
///
/// Returns a `Result` indicating whether the "sup" service is
/// required or not. The result is `Ok(true)` if the service is
/// required, and `Ok(false)` if it is not required. If there is an
/// error while accessing the configuration section or key, an `Err`
/// variant is returned with the specific error information.  This
/// would usually be a case of the HEARTBEAT section missing in the
/// `config`.
///
/// # Example
///
/// ```rust
/// use crate::Config;
///
/// let config = Config::load_from_file("config.cfg").unwrap();
/// let requires_sup = requires_sup(&config).unwrap();
///
/// if requires_sup {
///     println!("The 'sup' service is required.");
/// } else {
///     println!("The 'sup' service is not required.");
/// }
/// ```
fn requires_sup(config: &Config) -> Result<bool> {
    Ok(!config
        .section(section::HEARTBEAT)?
        .has_key(key::TARGET_ENDPOINT))
}

#[tokio::main()]
async fn main() -> Result<()> {
    let logger = Rc::new(LocalLogger::new(APP_ID));
    let mut config = Config::new();
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CONFIG_FILE_NAME.to_owned());
    logger.log(Info, &format!("Load config from path: {}", config_path));
    config
        .section_mut(section::HEARTBEAT)
        .load_from_path(&config_path)?;

    if requires_sup(&config)? {
        let mut path = dirs::config_dir().expect("no config directory in this platform");
        path.push("sup");
        path.push("sup.cfg");
        logger.log(Info, &format!("sup config: {}", path.to_string_lossy()));
        config.section_mut(section::SUP).load_from_path(&path)?;
        main_impl(config, logger).await
    } else {
        main_impl(config, logger).await
    }
}
