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
    primitives::{B256, fixed_bytes, address},
    providers::ProviderBuilder,
};
use eyre::{WrapErr, Result};
use prettytable::{Row, Cell, Table};

const RPC: &str = "http://localhost:8547";
const KEY: B256 = fixed_bytes!("0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659");

#[tokio::main]
async fn main() -> Result<()> {
    let programs = vec![
        ("opt-3", address!("0xe78b46ae59984d11a215b6f84c7de4cb111ef63c")),
        ("opt-s", address!("0xc6464a3072270a3da814bb0ec2907df935ff839d")),
    ];

    let methods = vec![
        ("number()", vec![]),
        ("setNumber(uint)", vec!["0xdeadbeef"]),
        ("mulNumber(uint)", vec!["0xdeadbeef"]),
        ("addNumber(uint)", vec!["0xdeadbeef"]),
        ("increment()", vec![]),
    ];

    let url = RPC.parse().wrap_err("failed to parse RPC")?;
    let provider = ProviderBuilder::new().on_http(url);

    let mut table = Table::new();

    let mut header = vec![Cell::new("Method")];
    for (program_name, _) in programs.iter() {
        header.push(Cell::new(program_name));
    }
    table.add_row(Row::new(header));

    for (signature, arguments) in methods.iter() {
        let mut row = vec![Cell::new(signature)];
        for (_, program_address) in programs.iter() {
            let calldata = stylus_ink_bench::generate_calldata(signature, arguments)?;
            let tx = stylus_ink_bench::send_tx(&provider, &KEY, program_address.clone(), calldata).await?;
            let ink = stylus_ink_bench::get_ink_usage(&RPC, &tx).await?;
            row.push(Cell::new(&format!("{}.{} gas", ink / 10_000, ink % 10_000)));
        }
        table.add_row(Row::new(row));
    }

    table.printstd();
    Ok(())
}
