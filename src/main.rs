mod checker;
mod crawler;
mod report;

use clap::Parser;
use checker::check_all_links;
use crawler::crawl;
use report::{print_report, save_to_file};

// ─────────────────────────────────────────────
// Args
//
// clap's derive macro turns this struct into a
// full CLI argument parser automatically.
// ─────────────────────────────────────────────
#[derive(Parser, Debug)]
#[command(
    name = "link-checker",
    about = "Crawls a URL and checks every link found on the page",
    version = "0.1.0"
)]
struct Args {
    /// The URL to crawl for links
    #[arg(short, long)]
    url: String,

    /// Max number of concurrent link checks (default: 10)
    #[arg(short, long, default_value_t = 10)]
    concurrency: usize,

    /// Optional path to save broken links as a .csv file
    #[arg(short, long)]
    output: Option<String>,
}

// ─────────────────────────────────────────────
// main
//
// #[tokio::main] is a macro that:
//   1. Creates a tokio multi-threaded runtime
//   2. Runs our async fn main() inside it
//   3. Blocks the OS thread until it completes
//
// Without this macro, `async fn main()` would
// not compile — Rust has no built-in async
// runtime, tokio provides one.
// ─────────────────────────────────────────────
#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("🕷️  Link Checker");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🌐 Target      : {}", args.url);
    println!("⚡ Concurrency : {} simultaneous checks", args.concurrency);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // ── Step 1: Build a shared HTTP client ───────────────────────
    // We build it once here and pass a borrow to crawl().
    // checker.rs will build its own Arc-wrapped client internally
    // since it needs to distribute it across tasks.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("link-checker/0.1")
        .build()
        .expect("Failed to build HTTP client");

    // ── Step 2: Crawl the starting page for links ─────────────────
    // crawl() is async so we .await it. If it fails (bad URL,
    // network down) we print the error and exit cleanly.
    let links = match crawl(&client, &args.url).await {
        Ok(links) => links,
        Err(e) => {
            eprintln!("❌ Failed to fetch {}: {}", args.url, e);
            std::process::exit(1);
        }
    };

    if links.is_empty() {
        println!("⚠️  No links found on {}. Nothing to check.", args.url);
        return;
    }

    // ── Step 3: Check all links concurrently ─────────────────────
    // This is where tokio::spawn, Arc, and Semaphore all come into
    // play. See checker.rs for the detailed ownership comments.
    println!("⏳ Checking {} links with concurrency {}...\n", links.len(), args.concurrency);
    let results = check_all_links(links, args.concurrency).await;

    // ── Step 4: Print the report ──────────────────────────────────
    // We pass &results (borrow) — print_report only needs to read
    // the data, and we still need results below for save_to_file.
    print_report(&results);

    // ── Step 5: Optionally save broken links to CSV ───────────────
    if let Some(output_path) = args.output {
        if let Err(e) = save_to_file(&results, &output_path) {
            eprintln!("❌ Failed to save report: {}", e);
        }
    }
}