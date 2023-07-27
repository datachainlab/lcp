//! ibc-proto library gives the developer access to the Cosmos SDK IBC proto-defined structs.

// Todo: automate the creation of this module setup based on the dots in the filenames.
// This module setup is necessary because the generated code contains "super::" calls for dependencies.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(warnings, trivial_casts, trivial_numeric_casts, unused_import_braces)]
#![allow(clippy::large_enum_variant)]
#![allow(rustdoc::bare_urls)]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate core as std;

#[macro_export]
macro_rules! include_proto {
    ($path:literal) => {
        include!(concat!("prost/", $path));
    };
}

/// The version (commit hash) of IBC Go used when generating this library.
pub const IBC_GO_COMMIT: &str = include_str!("IBC_GO_COMMIT");

/// Protobuf-encoded file descriptor set for all message types, used for gRPC reflection.
#[cfg(feature = "server")]
pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!("descriptor.bin");

pub use ibc_proto::{cosmos, google};

pub mod ibc {
    pub use ibc_proto::ibc::core;
    pub mod lightclients {
        pub mod lcp {
            pub mod v1 {
                include_proto!("ibc.lightclients.lcp.v1.rs");
            }
        }
    }
}

pub use ibc_proto::ics23;

pub mod lcp {
    pub mod service {
        pub mod enclave {
            pub mod v1 {
                include_proto!("lcp.service.enclave.v1.rs");
            }
        }
        pub mod elc {
            pub mod v1 {
                include_proto!("lcp.service.elc.v1.rs");
            }
        }
    }
}
