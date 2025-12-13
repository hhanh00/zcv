use crate::{
    ZCVResult,
    rpc::{BlockId, BlockRange, PoolType, compact_tx_streamer_client::CompactTxStreamerClient},
};
use tonic::{
    Request,
    transport::{Channel, Endpoint},
};

pub type Client = CompactTxStreamerClient<Channel>;

pub async fn connect(url: &str) -> ZCVResult<Client> {
    let ep = Endpoint::from_shared(url.to_string())?;
    let client = CompactTxStreamerClient::connect(ep).await?;
    Ok(client)
}

pub async fn get_blocks(client: &mut Client, start: u32, end: u32) -> ZCVResult<()> {
    let mut blocks = client.get_block_range(Request::new(BlockRange {
        start: Some(BlockId {
            height: (start + 1) as u64,
            hash: vec![],
        }),
        end: Some(BlockId {
            height: end as u64,
            hash: vec![],
        }),
        pool_types: vec![PoolType::Orchard.into()],
    })).await?.into_inner();

    while let Some(block) = blocks.message().await? {
        for tx in block.vtx {
            println!("{}", tx.actions.len());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use crate::lwd::{connect, get_blocks};

    #[tokio::test]
    async fn test_get_blocks() -> Result<()> {
        let mut client = connect("https://zec.rocks").await?;
        get_blocks(&mut client, 3_140_000, 3_160_000).await?;
        Ok(())
    }
}
