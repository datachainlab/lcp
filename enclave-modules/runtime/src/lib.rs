#![no_std]
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;
extern crate sgx_types;

use ctor::ctor;
pub(crate) use store::get_store;

mod ecalls;
mod errors;
mod key_manager;
mod light_client;
mod store;

#[ctor]
fn init_logger() {
    env_logger::init();
}
