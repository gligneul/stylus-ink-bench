// Copyright 2025 Gabriel Q Ligneul
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use alloy::{
    primitives::{Address, B256},
    providers::ProviderBuilder,
};
use clap::Parser;
use eyre::{WrapErr, Result};

/// Benchmark the Ink usage of a Stylus transaction
#[derive(Parser)]
#[command(about)]
struct Cli {
    /// Ethereum Client RPC
    #[arg(short, long, default_value = "http://localhost:8547")]
    rpc: String,

    /// Private key of the sender
    #[arg(short, long)]
    key: B256,

    /// Address of the Stylus program
    #[arg(short, long)]
    program: Address,

    /// Function signature
    #[arg(short, long)]
    signature: String,

    /// Function arguments
    #[arg(short, long)]
    args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let rpc = cli.rpc.parse().wrap_err("failed to parse RPC")?;
    let provider = ProviderBuilder::new().on_http(rpc);

    let calldata = stylus_ink_bench::generate_calldata(&cli.signature, cli.args)?;
    let tx = stylus_ink_bench::send_tx(&provider, &cli.key, cli.program, calldata).await?;
    let ink = stylus_ink_bench::get_ink_usage(&cli.rpc, &tx).await?;

    println!("{} ink", ink);
    println!("{} gas", ink as f64 / 10_000.0);

    Ok(())
}
