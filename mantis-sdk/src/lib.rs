use std::fmt::Display;
use std::future::Future;
use std::time::Duration;

use rand::Rng;
use tokio::time::sleep;
use tracing::{error, warn};

pub mod auction;
pub mod ethereum;
pub mod solana;

pub async fn retry<F, Fut, T, E>(mut function: F, max_tries: u32) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: Display,
{
    let mut count = 0;
    loop {
        count += 1;
        match function().await {
            Ok(result) => {
                return Ok(result);
            }
            Err(err) => {
                if count >= max_tries {
                    error!("Failed after {} tries: {:#}", count, err);
                    return Err(err);
                }
                warn!("Try {} of {} failed: {:#}", count, max_tries, err);
                sleep(Duration::from_secs(1) * count).await;
            }
        }
    }
}

pub fn random_intent_id() -> u64 {
    rand::thread_rng().gen_range(100_000_000_000..=999_999_999_999)
}
