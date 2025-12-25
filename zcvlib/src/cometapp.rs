use anyhow::Result;
use zcvlib::server::run_cometbft_app;

pub fn main() -> Result<()> {
    run_cometbft_app(26658)?.join().unwrap();
    Ok(())
}
