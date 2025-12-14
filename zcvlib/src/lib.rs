pub mod error;
pub mod pod;
pub mod context;
pub mod db;
pub mod lwd;

#[path = "cash.z.wallet.sdk.rpc.rs"]
pub mod rpc;
// or tonic::include_proto!("cash.z.wallet.sdk.rpc");

pub use error::ZCVResult;

#[macro_export]
macro_rules! tiu {
    ($x: expr) => {
        $x.try_into().unwrap()
    };
}
