pub mod error;
pub mod pod;
pub mod context;
pub mod db;
pub mod election;
pub mod lwd;
pub mod balance;
pub mod ballot;

#[path = "cash.z.wallet.sdk.rpc.rs"]
pub mod rpc;
// or tonic::include_proto!("cash.z.wallet.sdk.rpc");

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
