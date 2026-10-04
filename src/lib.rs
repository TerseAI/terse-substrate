#![doc = include_str!("../README.md")]

// Preserve upstream documentation verbatim.
#[allow(clippy::doc_lazy_continuation)]
mod generated {
    tonic::include_proto!("ateapi");
}

pub use generated::*;

mod client;
mod credentials;
mod routing;
mod snapshot;

pub use client::{Client, ConnectionOptions, Readiness};
pub use credentials::{Credentials, FileCredentials};
pub use routing::{TARGET_ACTOR_HEADER, target_actor_header, valid_resource_name};
pub use snapshot::{golden_tag, validate_tag};
