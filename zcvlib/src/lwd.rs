use tonic::transport::{Channel, Endpoint};
use crate::{ZCVResult, rpc::compact_tx_streamer_client::CompactTxStreamerClient};

pub type Client = CompactTxStreamerClient<Channel>;

pub async fn connect(url: &str) -> ZCVResult<Client> {
    let ep = Endpoint::from_shared(url.to_string())?;
    let client = CompactTxStreamerClient::connect(ep).await?;
    Ok(client)
}
