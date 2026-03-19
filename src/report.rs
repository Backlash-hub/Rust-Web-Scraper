use std::fs::File;
use std::io::{BufWriter, Write};

// ─────────────────────────────────────────────
// Data struct shared across the whole project.
// Derives Clone so tasks can hand copies around
// without fighting the borrow checker.
// ─────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct LinkResult {
    pub url: String,
    pub status_code: Option<u16>,
    pub is_ok: bool,
    pub error_msg: Option<String>,
}

impl LinkResult {
    // Convenience constructor for a successful check
    pub fn success(url: String, status_code: u16) -> Self {
        Self {
            url,
            status_code: Some(status_code),
            is_ok: true,
            error_msg: None,
        }
    }

    // Convenience constructor for a failed check
    pub fn failure(url: String, status_code: Option<u16>, error_msg: String) -> Self {
        Self {
            url,
            status_code,
            is_ok: false,
            error_msg: Some(error_msg),
        }
    }
}

// ─────────────────────────────────────────────
// print_report
//
// Takes a shared reference to all results and
// prints a formatted summary to stdout.
//
// Ownership note: we borrow &Vec<LinkResult>
// here — no need to take ownership since we are
// only reading the data, not consuming it.
// ─────────────────────────────────────────────
pub fn print_report(results: &Vec<LinkResult>) {
    let total = results.len();
    let ok_count = results.iter().filter(|r| r.is_ok).count();
    let broken_count = total - ok_count;

    println!("\n{}", "=".repeat(60));
    println!(" LINK CHECK REPORT");
    println!("{}", "=".repeat(60));
    println!(" Total checked : {}", total);
    println!(" ✅ OK          : {}", ok_count);
    println!(" ❌ Broken      : {}", broken_count);
    println!("{}", "=".repeat(60));

    if broken_count > 0 {
        println!("\n BROKEN LINKS:\n");
        for result in results.iter().filter(|r| !r.is_ok) {
            let status = result
                .status_code
                .map(|s| s.to_string())
                .unwrap_or_else(|| "N/A".to_string());

            let error = result
                .error_msg
                .as_deref()
                .unwrap_or("Unknown error");

            println!("  ❌ [{}] {}", status, result.url);
            println!("     └─ {}", error);
        }
    }

    println!("\n ALL RESULTS:\n");
    for result in results {
        let status = result
            .status_code
            .map(|s| s.to_string())
            .unwrap_or_else(|| "ERR".to_string());

        let icon = if result.is_ok { "✅" } else { "❌" };
        println!("  {} [{}] {}", icon, status, result.url);
    }

    println!();
}

// ─────────────────────────────────────────────
// save_to_file
//
// Writes only the broken links to a .csv file.
//
// Ownership note: returns a Result so the caller
// (main) can decide how to handle IO errors
// rather than panicking here.
// ─────────────────────────────────────────────
pub fn save_to_file(results: &Vec<LinkResult>, path: &str) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    // Write CSV header
    writeln!(writer, "url,status_code,error_msg")?;

    // Write only broken links
    for result in results.iter().filter(|r| !r.is_ok) {
        let status = result
            .status_code
            .map(|s| s.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        let error = result
            .error_msg
            .as_deref()
            .unwrap_or("Unknown error")
            .replace(',', ";"); // sanitize commas for CSV

        writeln!(writer, "{},{},{}", result.url, status, error)?;
    }

    println!("💾 Broken links saved to: {}", path);
    Ok(())
}