// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::remote_service::RemoteService;

use libra_config::config::{PersistableConfig, SafetyRulesConfig, SafetyRulesService};
use libra_temppath::TempPath;
use std::{net::SocketAddr, process::Child};

pub struct SpawnedProcess {
    handle: Child,
    server_addr: SocketAddr,
    _config_path: TempPath,
    network_timeout_ms: u64,
}

impl SpawnedProcess {
    pub fn new(config: &SafetyRulesConfig) -> Self {
        let mut config_path = TempPath::new();
        config_path.persist();
        config_path.create_as_file().unwrap();
        config.save_config(&config_path).unwrap();

        let service = &config.service;
        let server_addr = if let SafetyRulesService::SpawnedProcess(process_config) = service {
            process_config.server_address()
        } else {
            panic!("Invalid SafeRulesService, expected SpawnedProcess.");
        };

        Self {
            handle: runner::run(&config_path.path()),
            server_addr,
            _config_path: config_path,
            network_timeout_ms: config.network_timeout_ms,
        }
    }
}

impl RemoteService for SpawnedProcess {
    fn server_address(&self) -> SocketAddr {
        self.server_addr
    }
    fn network_timeout_ms(&self) -> u64 {
        self.network_timeout_ms
    }
}

/// Kill SafetyRules process upon this object going out of scope
impl Drop for SpawnedProcess {
    fn drop(&mut self) {
        match self.handle.try_wait() {
            Ok(Some(_)) => {}
            _ => {
                if let Err(e) = self.handle.kill() {
                    panic!("Spawned SafetyRules process could not be killed: '{}'", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod runner {
    const BINARY: &str = "safety-rules";

    pub fn run(path: &std::path::Path) -> std::process::Child {
        let mut command = std::process::Command::new(workspace_builder::get_bin(BINARY));
        command
            .arg(path)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        command.spawn().unwrap()
    }
}

#[cfg(not(test))]
mod runner {
    pub fn run(_path: &std::path::Path) -> std::process::Child {
        panic!("Not supported outside of testing");
    }
}
