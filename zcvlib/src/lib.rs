#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod error;
pub mod pod;
pub mod context;
pub mod db;
pub mod lwd;
pub mod pir;
pub mod balance;
pub mod ballot;
pub mod vote;
pub mod api;

#[cfg(feature = "graphql")]
pub mod voter;

#[cfg(feature = "server")]
pub mod server;

#[cfg(any(feature = "client", feature = "server"))]
#[path = "cash.z.wallet.sdk.rpc.rs"]
pub mod rpc;

#[cfg(any(feature = "client", feature = "server"))]
#[path = "cash.z.vote.sdk.rpc.rs"]
pub mod vote_rpc;

// pub mod frb_generated;

pub use error::ZCVResult;
pub use error::Error as ZCVError;

#[macro_export]
macro_rules! tiu {
    ($x: expr) => {
        $x.try_into().unwrap()
    };
}

#[cfg(test)]
pub mod tests;
