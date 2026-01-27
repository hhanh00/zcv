pub mod error;
pub mod pod;
pub mod context;
pub mod db;
pub mod election;
pub mod lwd;
pub mod balance;
pub mod ballot;
pub mod server;

#[path = "cash.z.wallet.sdk.rpc.rs"]
pub mod rpc;
// pub mod rpc {
//     tonic::include_proto!("cash.z.wallet.sdk.rpc");
// }

#[path = "cash.z.vote.sdk.rpc.rs"]
pub mod vote_rpc;
// pub mod vote_rpc {
//     tonic::include_proto!("cash.z.vote.sdk.rpc");
// }

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
