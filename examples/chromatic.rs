// SPDX-License-Identifier: GPL-3.0-only
//! Generate and inspect one Chromatic text packet on the host.
use chromatic_client::{Button, Packet};
use pocket_search::{Document, SearchIndex};
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: cargo run --example chromatic -- <documents.json> <query>".into());
    }
    let documents: Vec<Document> = serde_json::from_str(&fs::read_to_string(&args[0])?)?;
    let index = SearchIndex::new(documents);
    let bytes = index
        .chromatic_result(&args[1], 0)
        .map_err(|error| format!("{error:?}"))?;
    let packet = Packet::decode(&bytes).map_err(|error| format!("{error:?}"))?;
    println!(
        "Host protocol check: {} bytes, no hardware connected",
        bytes.len()
    );
    println!("{}", packet.title);
    for line in &packet.lines[..packet.line_count] {
        println!("{line}");
    }
    if let chromatic_client::Action::Open(id) = packet.action(Button::A) {
        let bytes = index
            .chromatic_page(id as usize, 0)
            .map_err(|error| format!("{error:?}"))?;
        let page = Packet::decode(&bytes).map_err(|error| format!("{error:?}"))?;
        println!(
            "Reader: page {}/{} ({} bytes)",
            page.page_index + 1,
            page.page_count,
            bytes.len()
        );
        for line in &page.lines[..page.line_count] {
            println!("{line}");
        }
    }
    Ok(())
}
