#![doc = include_str!("../README.md")]

// Preserve upstream documentation verbatim.
#[allow(clippy::doc_lazy_continuation)]
mod generated {
    tonic::include_proto!("ateapi");
}

pub use generated::*;
