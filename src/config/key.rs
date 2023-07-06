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

/// The key name for the COMMAND configuration item.
pub(crate) static COMMAND: &str = "COMMAND";

/// The key name for the COMMS-TIMEOUT configuration item.
pub(crate) static COMMS_TIMEOUT: &str = "COMMS-TIMEOUT";

/// The key name for the ENDPOINT configuration item.
pub(crate) static ENDPOINT: &str = "ENDPOINT";

/// The key name for the HEARTBEAT-INTERVAL configuration item.
pub(crate) static HEARTBEAT_INTERVAL: &str = "HEARTBEAT-INTERVAL";

/// The key name for the MAX-RETRIES configuration item.
pub(crate) static MAX_RETRIES: &str = "MAX-RETRIES";

/// The key name for the RETRY-INTERVAL configuration item.
pub(crate) static RETRY_INTERVAL: &str = "RETRY-INTERVAL";

/// The key name for the TARGET-ENDPOINT configuration item.
pub(crate) static TARGET_ENDPOINT: &str = "TARGET-ENDPOINT";

/// The key name for the WORKING-DIRECTORY configuration item.
pub(crate) static WORKING_DIRECTORY: &str = "WORKING-DIRECTORY";
