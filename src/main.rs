use pocket_search::{Document, SearchIndex};
use serde_json::json;
use std::{env, fs, process};
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
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
