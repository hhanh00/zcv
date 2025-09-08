use std::sync::LazyLock;

use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::{app::AppState, rpc::compact_tx_streamer_client::CompactTxStreamerClient};

#[path ="cash.z.wallet.sdk.rpc.rs"]
pub mod rpc;
pub mod api;
pub mod app;
pub mod db;
mod frb_generated;

pub type Hash = [u8; 32];
pub type Client = CompactTxStreamerClient<Channel>;

pub static APPSTATE: LazyLock<Mutex<AppState>> =
    LazyLock::new(|| Mutex::new(AppState::default()));
