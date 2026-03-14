use std::time::Duration;

pub const MAX_RETRIES: u32 = 4;
pub const BASE_BACKOFF_SECS: u64 = 5;

/// Parses the suggested retry delay from the 429 response body, falling back
/// to exponential backoff based on the attempt number.
pub fn retry_delay_secs(body: &str, attempt: u32) -> u64 {
    if let Some(start) = body.find("retry in ") {
        let after = &body[start + 9..];
        if let Some(end) = after.find('s') {
            if let Ok(secs) = after[..end].trim().parse::<f64>() {
                return (secs.ceil() as u64) + 1;
            }
        }
    }
    BASE_BACKOFF_SECS * (2u64.pow(attempt))
}

/// Logs the rate-limit message and sleeps for the appropriate backoff duration.
pub async fn sleep_for_retry(body: &str, attempt: u32, provider_name: &str) {
    let wait = retry_delay_secs(body, attempt);
    eprintln!(
        "Rate limited by {} — waiting {wait}s before retry {}/{MAX_RETRIES}...",
        provider_name,
        attempt + 1
    );
    tokio::time::sleep(Duration::from_secs(wait)).await;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_parses_suggested_seconds() {
        assert_eq!(retry_delay_secs("Please retry in 6.837s.", 0), 8);
    }

    #[test]
    fn retry_delay_falls_back_to_exponential_backoff() {
        assert_eq!(retry_delay_secs("no hint here", 0), 5);
        assert_eq!(retry_delay_secs("no hint here", 1), 10);
        assert_eq!(retry_delay_secs("no hint here", 2), 20);
    }
}
