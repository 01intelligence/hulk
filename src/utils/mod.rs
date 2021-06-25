use std::time::SystemTime;

use rand::rngs::StdRng;
pub use rand::Rng;
use rand::SeedableRng;
use tokio::time::{timeout, Duration, Instant};

pub fn rng_seed_now() -> StdRng {
    StdRng::seed_from_u64(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64, // max 584 years
    )
}

pub async fn sleep(timeout: Duration, rng: Option<&mut StdRng>) {
    let rand = rng.map_or_else(|| rng_seed_now().gen::<f64>(), |rng| rng.gen::<f64>());
    tokio::time::sleep(timeout.mul_f64(rand)).await;
}

pub async fn sleep_until(deadline: Instant, timeout: Duration, rng: Option<&mut StdRng>) {
    let rand = rng.map_or_else(|| rng_seed_now().gen::<f64>(), |rng| rng.gen::<f64>());
    let sleep_deadline = Instant::now() + timeout.mul_f64(rand);
    if sleep_deadline < deadline {
        tokio::time::sleep_until(sleep_deadline).await;
    }
}

pub fn ceil_frac(mut numerator: isize, mut denominator: isize) -> isize {
    if denominator == 0 {
        // do nothing on invalid input
        return 0;
    }
    // Make denominator positive
    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }
    let mut ceil = numerator / denominator;
    if numerator > 0 && numerator % denominator != 0 {
        ceil += 1;
    }
    ceil
}
