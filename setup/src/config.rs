use clap::Parser;
use redact::Secret;
use serde::{Deserialize, Serialize};

#[derive(Parser, Serialize, Deserialize, Debug)]
pub struct CLIArgs {
    #[clap(short, long, value_parser)]
    pub config_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub name: String,
    pub port: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(serialize_with = "redact::serde::redact_secret")]
    pub authkey: Secret<String>,
    pub uid: u32,
    pub nodes: Vec<Node>,
    pub datadir: String,
}
