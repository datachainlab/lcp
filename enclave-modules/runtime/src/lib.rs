#![no_std]
#![allow(internal_features)]
#![feature(rustc_private)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::sync::atomic::{AtomicPtr, Ordering};
use core::{mem, ptr};

pub use alloc::alloc::*;
pub use sgx_alloc::System;

pub use ecalls::{ecall_execute_command, set_environment};
pub use enclave_environment::{Environment, MapLightClientRegistry};
/// re-export
pub use sgx_trts;
pub use sgx_types;

mod ecalls;
mod errors;

/// global allocator and panic handler for enclave
///
/// This is a fork of the `sgx_no_tstd` crate, with the following change:
/// - The `begin_panic_handler` function is added to logging the panic message before aborting.

#[cfg(not(test))]
#[global_allocator]
static ALLOC: sgx_alloc::System = sgx_alloc::System;

#[cfg(not(test))]
#[panic_handler]
fn begin_panic_handler(info: &core::panic::PanicInfo<'_>) -> ! {
    let _ = host_api::api::execute_command(host_api::ocall_commands::Command::Log(
        host_api::ocall_commands::LogCommand {
            msg: alloc::format!("[enclave] panic: {:?}\n", info).into_bytes(),
        },
    ));
    sgx_abort();
}

#[cfg(not(test))]
#[lang = "eh_personality"]
#[no_mangle]
unsafe extern "C" fn rust_eh_personality() {}

static HOOK: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

/// Registers a custom allocation error hook, replacing any that was previously registered.
///
/// The allocation error hook is invoked when an infallible memory allocation fails, before
/// the runtime aborts. The default hook prints a message to standard error,
/// but this behavior can be customized with the [`set_alloc_error_hook`] and
/// [`take_alloc_error_hook`] functions.
///
/// The hook is provided with a `Layout` struct which contains information
/// about the allocation that failed.
///
/// The allocation error hook is a global resource.
pub fn set_alloc_error_hook(hook: fn(Layout)) {
    HOOK.store(hook as *mut (), Ordering::SeqCst);
}

/// Unregisters the current allocation error hook, returning it.
///
/// *See also the function [`set_alloc_error_hook`].*
///
/// If no custom hook is registered, the default hook will be returned.
pub fn take_alloc_error_hook() -> fn(Layout) {
    let hook = HOOK.swap(ptr::null_mut(), Ordering::SeqCst);
    if hook.is_null() {
        default_alloc_error_hook
    } else {
        unsafe { mem::transmute(hook) }
    }
}

fn default_alloc_error_hook(_layout: Layout) {}

#[cfg(not(test))]
#[alloc_error_handler]
pub fn rust_oom(layout: Layout) -> ! {
    let hook = HOOK.load(Ordering::SeqCst);
    let hook: fn(Layout) = if hook.is_null() {
        default_alloc_error_hook
    } else {
        unsafe { mem::transmute(hook) }
    };
    hook(layout);
    sgx_abort();
}

#[cfg(not(test))]
#[link(name = "sgx_trts")]
extern "C" {
    pub fn abort() -> !;
}

#[cfg(not(test))]
fn sgx_abort() -> ! {
    unsafe { abort() }
}

#[allow(unused_imports)]
mod prelude {
    pub use core::prelude::v1::*;

    // Re-export according to alloc::prelude::v1 because it is not yet stabilized
    // https://doc.rust-lang.org/src/alloc/prelude/v1.rs.html
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;

    pub use alloc::format;
    pub use alloc::vec;

    // Those are exported by default in the std prelude in Rust 2021
    pub use core::convert::{TryFrom, TryInto};
    pub use core::iter::FromIterator;
}

#[macro_export]
macro_rules! setup_runtime {
    ($func:block) => {
        use $crate::sgx_types::cfg_if;

        $crate::sgx_trts::global_ctors_object! {_init, _init_func = {
            $crate::set_environment((|| { $func })()).unwrap()
        }}

        #[no_mangle]
        pub unsafe extern "C" fn ecall_execute_command(
            command: *const u8,
            command_len: u32,
            output_buf: *mut u8,
            output_buf_maxlen: u32,
            output_len: &mut u32,
        ) -> u32 {
            $crate::ecall_execute_command(
                command,
                command_len,
                output_buf,
                output_buf_maxlen,
                output_len,
            ) as u32
        }
    };
}
