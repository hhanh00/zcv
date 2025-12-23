use tendermint_abci::Application;
use tendermint_proto::abci::{
    RequestCheckTx, RequestFinalizeBlock, RequestInfo, RequestPrepareProposal, ResponseCheckTx,
    ResponseFinalizeBlock, ResponseInfo, ResponsePrepareProposal,
};

#[derive(Clone)]
pub struct Server {}

impl Application for Server {
    fn info(&self, _request: RequestInfo) -> ResponseInfo {
        Default::default()
    }

    // Checks if a tx is structurally correct
    // Valid txs must not be rejected
    // But bad txs may be kept
    fn check_tx(&self, _request: RequestCheckTx) -> ResponseCheckTx {
        Default::default()
    }

    // Select txs to include in a block
    // Bad txs must be rejected
    // Valid txs may be excluded (for example, if the block is full)
    fn prepare_proposal(&self, request: RequestPrepareProposal) -> ResponsePrepareProposal {
        // Per the ABCI++ spec: if the size of RequestPrepareProposal.txs is
        // greater than RequestPrepareProposal.max_tx_bytes, the Application
        // MUST remove transactions to ensure that the
        // RequestPrepareProposal.max_tx_bytes limit is respected by those
        // transactions returned in ResponsePrepareProposal.txs.
        let RequestPrepareProposal {
            mut txs,
            max_tx_bytes,
            ..
        } = request;
        let max_tx_bytes: usize = max_tx_bytes.try_into().unwrap_or(0);
        let mut total_tx_bytes: usize = txs
            .iter()
            .map(|tx| tx.len())
            .fold(0, |acc, len| acc.saturating_add(len));
        while total_tx_bytes > max_tx_bytes {
            if let Some(tx) = txs.pop() {
                total_tx_bytes = total_tx_bytes.saturating_sub(tx.len());
            } else {
                break;
            }
        }
        ResponsePrepareProposal { txs }
    }

    // Process the block that was voted on by the validators
    fn finalize_block(&self, _request: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        Default::default()
    }
}
