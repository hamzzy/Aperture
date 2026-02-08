//! Retry utility with exponential backoff

use std::time::Duration;
use tracing::warn;

/// Retry an async operation with exponential backoff.
///
/// Returns `Ok` on first success, or the last `Err` after all attempts are exhausted.
/// Delays: `initial_delay`, `2 * initial_delay`, `4 * initial_delay`, ... capped at 30s.
pub async fn retry_with_backoff<F, Fut, T, E>(
    operation_name: &str,
    max_attempts: u32,
    initial_delay: Duration,
    mut f: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut delay = initial_delay;
    let mut last_err = None;

    for attempt in 1..=max_attempts {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                warn!(
                    "{} failed (attempt {}/{}): {}",
                    operation_name, attempt, max_attempts, e
                );
                last_err = Some(e);
                if attempt < max_attempts {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(30));
                }
            }
        }
    }

    Err(last_err.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_succeeds_first_try() {
        let result: Result<&str, String> =
            retry_with_backoff("test", 3, Duration::from_millis(1), || async { Ok("done") }).await;
        assert_eq!(result.unwrap(), "done");
    }

    #[tokio::test]
    async fn test_succeeds_after_retries() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        let result: Result<&str, String> =
            retry_with_backoff("test", 3, Duration::from_millis(1), move || {
                let counter = counter_clone.clone();
                async move {
                    let n = counter.fetch_add(1, Ordering::Relaxed);
                    if n < 2 {
                        Err(format!("fail #{}", n))
                    } else {
                        Ok("done")
                    }
                }
            })
            .await;
        assert_eq!(result.unwrap(), "done");
        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_all_attempts_fail() {
        let result: Result<(), String> =
            retry_with_backoff("test", 2, Duration::from_millis(1), || async {
                Err("always fails".to_string())
            })
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "always fails");
    }
}
