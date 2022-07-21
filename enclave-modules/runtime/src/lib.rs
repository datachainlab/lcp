#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;
extern crate sgx_types;

pub(crate) use crate::store::get_store;
use ctor::ctor;

mod ecalls;
mod errors;
mod light_client;
mod store;

#[ctor]
fn init_logger() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
}
