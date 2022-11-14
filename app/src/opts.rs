use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Opts {
    /// Path to the home directory
    #[clap(long = "home", help = "Path to LCP home directory")]
    pub home: Option<PathBuf>,
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

    pub fn get_store_path(&self) -> PathBuf {
        self.get_home().join("store")
    }
}
