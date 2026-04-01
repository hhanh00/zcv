use anyhow::Result;
use bigdecimal::{BigDecimal, Zero, num_bigint::BigInt};
use clap::Parser;
use serde::{Deserialize, Serialize};
use zcvlib::{
    api::simple::{collect_results, decode_ballots},
    context::Context,
    vote::VoteResultItem,
};

#[derive(Parser, Serialize, Deserialize, Debug)]
pub struct Config {
    #[clap(short, long, value_parser)]
    pub seed: String,
    #[clap(short, long, value_parser)]
    pub election_url: String,
    #[clap(short, long, value_parser)]
    pub db_path: Option<String>,
    #[clap(short, long, value_parser)]
    pub output: String,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .compact()
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let config = Config::parse();
    let Config {
        seed,
        election_url,
        db_path,
        output,
    } = config;
    let db_path = db_path.unwrap_or("count.db".to_string());

    let context = Context::new(&db_path, "", "", &election_url).await?;

    decode_ballots(seed, &context).await?;
    let tally_items = collect_results(&context).await?;
    let mut max_rows = 0;
    let mut max_cols = 0;
    for tally_item in tally_items.iter() {
        let VoteResultItem {
            idx_question,
            idx_answer,
            votes,
        } = *tally_item;
        if max_rows < idx_question {
            max_rows = idx_question;
        }
        if max_cols < idx_answer {
            max_cols = idx_answer;
        }
        tracing::info!("{idx_question} {idx_answer} {votes}");
    }
    let nrows = max_rows as usize + 1;
    let ncols = max_cols as usize;
    let mut results = vec![BigDecimal::zero(); nrows * ncols];

    for tally_item in tally_items {
        let VoteResultItem {
            idx_question,
            idx_answer,
            votes,
        } = tally_item;
        let v = BigDecimal::from_bigint(
            BigInt::from(votes), 8);
        results[idx_question as usize * ncols + idx_answer as usize - 1] = v;
    }

    let mut wtr = csv::Writer::from_writer(vec![]);
    for i in 0..nrows {
        let record: Vec<String> = results[i * ncols..(i + 1) * ncols]
            .iter()
            .map(|v| v.to_string())
            .collect();
        wtr.write_record(&record)?;
    }

    let contents = String::from_utf8(wtr.into_inner()?)?;
    std::fs::write(output, contents)?;
    Ok(())
}
