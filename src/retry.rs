use rand::Rng;
use reqwest::header::HeaderMap;
use std::time::Duration;

pub const MAX_RETRIES: u32 = 4;
pub const BASE_BACKOFF_SECS: u64 = 5;

/// Returns true if the given HTTP status code is worth retrying.
/// Covers rate limits (429) and common transient server errors (5xx).
pub fn should_retry(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// Returns true if the reqwest error represents a transient network condition
/// that is worth retrying (timeout, connection failure, or request send failure).
pub fn is_retryable_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}

/// Extracts an integer seconds value from the `Retry-After` HTTP header, if present.
/// Returns the parsed value plus 1 second of padding, or `None` if the header is
/// absent or cannot be parsed as a number.
pub fn parse_retry_after_header(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map(|secs| secs.ceil() as u64 + 1)
}

/// Returns the suggested retry delay in seconds, with the following priority:
///
/// 1. `header_secs` — value parsed from the `Retry-After` response header
/// 2. Body hint — parses "retry in Xs" from the response body text
/// 3. Exponential backoff with equal jitter — spreads concurrent retries to avoid
///    thundering-herd problems when multiple tasks hit the same rate limit simultaneously
pub fn retry_delay_secs(header_secs: Option<u64>, body: &str, attempt: u32) -> u64 {
    // Priority 1: Retry-After header
    if let Some(secs) = header_secs {
        return secs;
    }
    // Priority 2: body hint ("retry in Xs")
    if let Some(start) = body.find("retry in ") {
        let after = &body[start + 9..];
        if let Some(end) = after.find('s') {
            if let Ok(secs) = after[..end].trim().parse::<f64>() {
                return (secs.ceil() as u64) + 1;
            }
        }
    }
    // Priority 3: exponential backoff with equal jitter.
    // Equal jitter = half the cap plus a random value in [0, half], which guarantees
    // a meaningful minimum wait while still spreading concurrent retries.
    let cap = BASE_BACKOFF_SECS * (2u64.pow(attempt));
    let half = cap / 2;
    half + rand::thread_rng().gen_range(0..=half)
}

/// Logs the rate-limit message and sleeps for the appropriate backoff duration.
pub async fn sleep_for_retry(
    header_secs: Option<u64>,
    body: &str,
    attempt: u32,
    provider_name: &str,
) {
    let wait = retry_delay_secs(header_secs, body, attempt);
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
    fn retry_delay_header_takes_priority_over_body() {
        assert_eq!(retry_delay_secs(Some(15), "Please retry in 6.837s.", 0), 15);
    }

    #[test]
    fn retry_delay_parses_suggested_seconds_from_body() {
        assert_eq!(retry_delay_secs(None, "Please retry in 6.837s.", 0), 8);
    }

    #[test]
    fn retry_delay_exponential_backoff_with_jitter_is_in_range() {
        // attempt 0: cap=5, half=2, range [2, 5]
        for _ in 0..20 {
            let d = retry_delay_secs(None, "no hint here", 0);
            assert!((2..=5).contains(&d), "attempt 0 delay {d} out of [2, 5]");
        }
        // attempt 1: cap=10, half=5, range [5, 10]
        for _ in 0..20 {
            let d = retry_delay_secs(None, "no hint here", 1);
            assert!((5..=10).contains(&d), "attempt 1 delay {d} out of [5, 10]");
        }
        // attempt 2: cap=20, half=10, range [10, 20]
        for _ in 0..20 {
            let d = retry_delay_secs(None, "no hint here", 2);
            assert!(
                (10..=20).contains(&d),
                "attempt 2 delay {d} out of [10, 20]"
            );
        }
    }

    #[test]
    fn parse_retry_after_header_returns_none_when_absent() {
        let headers = HeaderMap::new();
        assert_eq!(parse_retry_after_header(&headers), None);
    }

    #[test]
    fn parse_retry_after_header_parses_integer_seconds() {
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "30".parse().unwrap());
        assert_eq!(parse_retry_after_header(&headers), Some(31));
    }

    #[test]
    fn parse_retry_after_header_parses_fractional_seconds() {
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "4.2".parse().unwrap());
        assert_eq!(parse_retry_after_header(&headers), Some(6)); // ceil(4.2)=5, +1=6
    }

    #[test]
    fn parse_retry_after_header_zero_returns_one_second_padding() {
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "0".parse().unwrap());
        assert_eq!(parse_retry_after_header(&headers), Some(1));
    }

    #[test]
    fn parse_retry_after_header_non_numeric_returns_none() {
        // HTTP-date and other non-numeric values are not parsed.
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            "Fri, 31 Dec 1999 23:59:59 GMT".parse().unwrap(),
        );
        assert_eq!(parse_retry_after_header(&headers), None);
    }

    #[test]
    fn parse_retry_after_header_whitespace_trimmed() {
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "  30  ".parse().unwrap());
        assert_eq!(parse_retry_after_header(&headers), Some(31));
    }

    #[test]
    fn retry_delay_body_hint_malformed_falls_back_to_backoff() {
        // "retry in" present but no number before "s" — should fall through to backoff.
        let d = retry_delay_secs(None, "retry in the future sometime", 0);
        assert!(
            (2..=5).contains(&d),
            "delay {d} should be in backoff range [2, 5]"
        );
    }

    #[test]
    fn retry_delay_body_hint_zero_seconds() {
        assert_eq!(retry_delay_secs(None, "retry in 0s", 0), 1);
    }

    #[test]
    fn retry_delay_higher_attempts_in_range() {
        // attempt 3: cap=40, half=20, range [20, 40]
        for _ in 0..20 {
            let d = retry_delay_secs(None, "no hint", 3);
            assert!(
                (20..=40).contains(&d),
                "attempt 3 delay {d} out of [20, 40]"
            );
        }
        // attempt 4: cap=80, half=40, range [40, 80]
        for _ in 0..20 {
            let d = retry_delay_secs(None, "no hint", 4);
            assert!(
                (40..=80).contains(&d),
                "attempt 4 delay {d} out of [40, 80]"
            );
        }
    }

    #[test]
    fn retry_delay_backoff_is_always_nonzero() {
        for attempt in 0..5 {
            for _ in 0..50 {
                let d = retry_delay_secs(None, "no hint", attempt);
                assert!(
                    d > 0,
                    "backoff delay should never be zero (attempt {attempt})"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Additional edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn should_retry_covers_expected_codes() {
        assert!(should_retry(429));
        assert!(should_retry(500));
        assert!(should_retry(502));
        assert!(should_retry(503));
        assert!(should_retry(504));
    }

    #[test]
    fn should_retry_rejects_success_and_client_errors() {
        assert!(!should_retry(200));
        assert!(!should_retry(201));
        assert!(!should_retry(400));
        assert!(!should_retry(401));
        assert!(!should_retry(403));
        assert!(!should_retry(404));
    }

    #[test]
    fn retry_delay_empty_body_uses_backoff() {
        let d = retry_delay_secs(None, "", 0);
        assert!((2..=5).contains(&d), "empty body delay {d} out of range");
    }

    #[test]
    fn retry_delay_header_value_of_one() {
        assert_eq!(retry_delay_secs(Some(1), "", 0), 1);
    }
}
