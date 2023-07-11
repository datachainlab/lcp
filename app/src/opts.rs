use anyhow::{anyhow, Result};
use clap::Parser;
use log::LevelFilter;
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Parser)]
pub struct Opts {
    /// Path to the home directory
    #[clap(long = "home", help = "Path to LCP home directory")]
    pub home: Option<PathBuf>,
    /// Verbosity level of the logger
    /// priority for setting log level:
    /// 1. command line option
    /// 2. environment variable
    #[clap(long = "log_level", help = "Verbosity level of the logger")]
    pub log_level: Option<String>,
}

impl Opts {
    pub fn get_home(&self) -> PathBuf {
        if let Some(home) = self.home.as_ref() {
            home.clone()
        } else {
            dirs::home_dir().unwrap().join(".lcp")
        }
    }

    pub fn default_enclave(&self) -> PathBuf {
        self.get_home().join("enclave.signed.so")
    }

    pub fn get_state_store_path(&self) -> PathBuf {
        self.get_home().join("state")
    }

    pub fn get_log_level_filter(&self) -> Result<Option<LevelFilter>> {
        if let Some(log_level) = self.log_level.as_ref() {
            Ok(Some(LevelFilter::from_str(log_level).map_err(|_| {
                anyhow!("Log level '{}' is not supported. The following levels are available: [OFF, ERROR, WARN, INFO, DEBUG, TRACE]", log_level)
            })?))
        } else {
            Ok(None)
        }
    }
}
