// SPDX-License-Identifier: GPL-3.0-only
use pocket_search::{Document, SearchIndex};
use serde_json::json;
use std::{env, fs, path::Path, process};
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args == ["--version"] {
        println!("pocket-search {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args == ["--build-info"] {
        println!(
            "{}",
            json!({"version":env!("CARGO_PKG_VERSION"),
            "commit":option_env!("POCKET_BUILD_COMMIT").unwrap_or("unknown"),
            "protocol_version":1})
        );
        return Ok(());
    }
    if args
        .first()
        .is_some_and(|arg| arg == "crawl" || arg == "__crawl_worker" || arg == "serve-crawl")
    {
        if args.len() != 3 {
            return Err("Usage: pocket-search crawl <config.json> <new-output-directory> | serve-crawl <output-directory> <bind-address:port>".into());
        }
        match args[0].as_str() {
            "__crawl_worker" => {
                pocket_search::crawler::worker(Path::new(&args[1]), Path::new(&args[2]))?
            }
            "crawl" => {
                let report =
                    pocket_search::crawler::crawl(Path::new(&args[1]), Path::new(&args[2]))?;
                println!(
                    "{}",
                    json!({"collected_pages":report.pages.len(),"requests":report.requests,"body_bytes_read":report.body_bytes_read,"stopped":report.stopped,"pending_pages":report.pending.len()})
                );
            }
            _ => pocket_search::server::serve(
                pocket_search::crawler::load(Path::new(&args[1]))?,
                &args[2],
            )?,
        }
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "serve") {
        if args.len() != 3 && !(args.len() == 5 && args[3] == "--html") {
            return Err("Usage: pocket-search serve <documents.json> <bind-address:port> [--html <manifest.json>]".into());
        }
        let index = pocket_search::server::load(&args[1])?;
        let index = if args.len() == 5 {
            pocket_search::dom::append(index, &args[4])?
        } else {
            index
        };
        pocket_search::server::serve(index, &args[2])?;
        return Ok(());
    }
    if args.len() != 2 {
        return Err("Usage: pocket-search <documents.json> <query>".into());
    }
    let documents: Vec<Document> = serde_json::from_str(&fs::read_to_string(&args[0])?)?;
    let index = SearchIndex::new(documents);
    let hits = index.search(&args[1], 10);
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"indexed_pages":index.document_count(),"results":hits})
        )?
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(2);
    }
}
