use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::report::LinkResult;

pub async fn check_link(client: Arc<reqwest::Client>, url: String) -> LinkResult {
    // Try HEAD first — faster, no body download
    let result = client
        .head(&url)
        .send()
        .await;

    match result {
        Ok(response) => {
            let status = response.status().as_u16();

            // Treat 2xx and 3xx as OK, everything else as broken
            if response.status().is_success() || response.status().is_redirection() {
                LinkResult::success(url, status)
            } else {
                LinkResult::failure(
                    url,
                    Some(status),
                    format!("HTTP error: {}", status),
                )
            }
        }

        // HEAD not supported by some servers — fall back to GET
        Err(_) => {
            let fallback = client.get(&url).send().await;

            match fallback {
                Ok(response) => {
                    let status = response.status().as_u16();

                    if response.status().is_success() || response.status().is_redirection() {
                        LinkResult::success(url, status)
                    } else {
                        LinkResult::failure(
                            url,
                            Some(status),
                            format!("HTTP error: {}", status),
                        )
                    }
                }

                // Both HEAD and GET failed — network error, DNS failure, timeout, etc.
                Err(e) => LinkResult::failure(url, None, e.to_string()),
            }
        }
    }
}


pub async fn check_all_links(
    links: Vec<String>,
    concurrency: usize,
) -> Vec<LinkResult> {
    // Wrap client in Arc so every task can share it
    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("link-checker/0.1")
            .build()
            .expect("Failed to build HTTP client"),
    );

    // Semaphore enforces a ceiling on concurrent tasks.
    // acquire() blocks until a permit is available,
    // then releases it automatically when _permit is dropped.
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut handles = vec![];

    for link in links {
        // Clone the Arc pointers BEFORE the move closure.
        // Arc::clone() is cheap — it only increments a reference
        // count, it does NOT clone the client or semaphore data.
        let client_clone = Arc::clone(&client);
        let sem_clone = Arc::clone(&semaphore);

        // tokio::spawn takes an async block and schedules it.
        // `move` transfers ownership of client_clone, sem_clone,
        // and link into the task.
        let handle = tokio::spawn(async move {
            // acquire() waits until concurrency slot is free.
            // The returned permit is held until `_permit` drops
            // at the end of this block — automatic release.
            let _permit = sem_clone
                .acquire()
                .await
                .expect("Semaphore closed unexpectedly");

            check_link(client_clone, link).await
            // _permit drops here → frees a concurrency slot
        });

        handles.push(handle);
    }

    // Collect results from all tasks.
    // JoinHandle::await gives us Result<T, JoinError>.
    // We unwrap JoinError (task panic) — in production you'd
    // want to handle this more gracefully.
    let mut results = vec![];
    for handle in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => eprintln!("Task panicked: {}", e),
        }
    }

    results
}