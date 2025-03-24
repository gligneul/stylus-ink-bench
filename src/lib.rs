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
    dyn_abi::{DynSolValue, JsonAbiExt, Specifier},
    json_abi::Function,
    network::{EthereumWallet, TransactionBuilder},
    primitives::{Address, B256},
    providers::Provider,
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use eyre::{bail, Result, WrapErr};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostioTraceInfo {
    name: String,
    args: String, // Using String to hold hex-encoded bytes
    outs: String, // Using String to hold hex-encoded bytes
    start_ink: u64,
    end_ink: u64,
    address: Option<String>,
    steps: Option<Vec<HostioTraceInfo>>, // Recursively defining steps
}

/// Parse the method signature and arguments, returning the calldata.
pub fn generate_calldata(signature: &str, args: Vec<String>) -> Result<Vec<u8>> {
    let func = Function::parse(signature).wrap_err("failed to parse function signature")?;

    // Check args and params len
    let params = &func.inputs;
    if args.len() != params.len() {
        bail!(
            "mismatch number of arguments (want {}; got {})",
            params.len(),
            args.len()
        );
    }

    // Parse arguments
    let mut values = Vec::<DynSolValue>::with_capacity(args.len());
    for (arg, param) in args.iter().zip(params) {
        let ty = param
            .resolve()
            .wrap_err_with(|| format!("could not resolve arg: {param}"))?;
        let value = ty
            .coerce_str(arg)
            .wrap_err_with(|| format!("could not parse arg: {param}"))?;
        values.push(value);
    }

    // Generate calldata
    func.abi_encode_input(&values)
        .wrap_err("failed to encode input")
}

/// Send a transaction calling the program with the given calldata, wait for it to be confirmed,
/// and return the transaction hash.
pub async fn send_tx(
    provider: &dyn Provider,
    key: &B256,
    program: Address,
    calldata: Vec<u8>,
) -> Result<B256> {
    let signer = PrivateKeySigner::from_bytes(key).wrap_err("failed to create signer")?;
    let sender = signer.address();
    let wallet = EthereumWallet::from(signer);
    let tx = TransactionRequest::default()
        .with_to(program)
        .with_nonce(provider.get_transaction_count(sender).await?)
        .with_chain_id(provider.get_chain_id().await?)
        .with_input(calldata)
        .with_gas_limit(30_000_000)
        .with_max_priority_fee_per_gas(1_000_000_000)
        .with_max_fee_per_gas(20_000_000_000);
    let tx_envelope = tx
        .build(&wallet)
        .await
        .wrap_err("failed to create transaction")?;
    let receipt = provider
        .send_tx_envelope(tx_envelope)
        .await
        .wrap_err("failed to send transaction")?
        .get_receipt()
        .await
        .wrap_err("failed to wait for transaction")?;
    Ok(receipt.transaction_hash)
}

/// Trace the transaction and return the ink usage.
pub async fn get_ink_usage(rpc: &str, tx: &B256) -> Result<u64> {
    let client = Client::new();
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "debug_traceTransaction",
        "params": [
            format!("{}", tx),
            { "tracer": "stylusTracer" }
        ],
        "id": "1"
    });
    let response = client
        .post(rpc)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .wrap_err("failed to trace transaction")?;
    let response_text = response.text().await?;
    let parsed_response: serde_json::Value =
        serde_json::from_str(&response_text).wrap_err("failed to parse json response")?;
    let Some(result) = parsed_response.get("result") else {
        bail!("failed to get result from response");
    };
    let hostio_trace: Vec<HostioTraceInfo> =
        serde_json::from_value(result.clone()).wrap_err("failed to parse hostio trace")?;
    if hostio_trace.len() < 3 {
        bail!("hostio trace is empty");
    }
    let start_ink = hostio_trace[0].start_ink;
    let end_ink = hostio_trace[hostio_trace.len() - 2].end_ink;
    Ok(start_ink - end_ink)
}
