use scraper::{Html, Selector};
use url::Url;

// ─────────────────────────────────────────────
// fetch_page
//
// Makes an async GET request and returns the
// raw HTML body as a String.
//
// Ownership note: `url` is taken as &str — we
// only need to read it, not own it. The returned
// String is owned by the caller.
//
// The `?` operator propagates reqwest::Error up
// to the caller rather than panicking here.
// ─────────────────────────────────────────────
pub async fn fetch_page(client: &reqwest::Client, url: &str) -> Result<String, reqwest::Error> {
    println!("🔍 Fetching: {}", url);

    let response = client
        .get(url)
        .send()
        .await?;   // .await yields control back to the tokio runtime while waiting for the response

    let body = response
        .text()
        .await?;   // second .await to read the response body stream

    Ok(body)
}

// ─────────────────────────────────────────────
// extract_links
//
// Parses raw HTML and pulls out all href values
// from <a> tags. Resolves relative URLs against
// the base URL so every link is absolute.
//
// Ownership note: takes &str borrows for both
// params — we parse into owned Strings in the
// iterator chain and return a Vec<String> that
// the caller owns.
// ─────────────────────────────────────────────
pub fn extract_links(html: &str, base_url: &str) -> Vec<String> {
    let document = Html::parse_document(html);

    // CSS selector for any <a> tag that has an href attribute
    let selector = Selector::parse("a[href]").unwrap();

    // Parse the base URL once so we can resolve relative paths
    let base = match Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return vec![], // if base URL is invalid, return nothing
    };

    document
        .select(&selector)
        .filter_map(|element| {
            // .attr() returns Option<&str> — filter_map discards None values
            element.value().attr("href")
        })
        .filter_map(|href| {
            // resolve_url handles relative, absolute, and protocol-relative hrefs
            resolve_url(href, &base)
        })
        .filter(|url| {
            // only keep http/https links — skip mailto:, javascript:, #anchors, etc.
            url.starts_with("http://") || url.starts_with("https://")
        })
        .collect()
}

// ─────────────────────────────────────────────
// resolve_url  (private helper)
//
// Turns a raw href string into an absolute URL.
//
// Ownership note: takes &str and &Url by borrow,
// returns an owned Option<String>.
// ─────────────────────────────────────────────
fn resolve_url(href: &str, base: &Url) -> Option<String> {
    // Case 1: already an absolute URL
    if let Ok(absolute) = Url::parse(href) {
        // Url::parse succeeds even for "mailto:foo" so check the scheme
        if absolute.scheme() == "http" || absolute.scheme() == "https" {
            return Some(absolute.to_string());
        }
        return None;
    }

    // Case 2: relative URL — join with base
    base.join(href)
        .ok()
        .map(|u| u.to_string())
}

// ─────────────────────────────────────────────
// crawl
//
// Top-level function called from main.
// Fetches the starting page, extracts all links,
// and deduplicates them before returning.
//
// Ownership note: returns owned Vec<String>.
// The client is borrowed — crawl does not need
// to own it, checker.rs will also need it.
// ─────────────────────────────────────────────
pub async fn crawl(client: &reqwest::Client, url: &str) -> Result<Vec<String>, reqwest::Error> {
    let html = fetch_page(client, url).await?;
    let mut links = extract_links(&html, url);

    // Deduplicate — sort first, then dedup (dedup only removes consecutive dupes)
    links.sort();
    links.dedup();

    println!("📎 Found {} unique links on {}\n", links.len(), url);

    Ok(links)
}