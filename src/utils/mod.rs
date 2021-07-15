mod str;
mod units;
use std::time::SystemTime;

use rand::rngs::StdRng;
pub use rand::Rng;
use rand::SeedableRng;
use tokio::time::{timeout, Duration, Instant};
pub use units::*;

pub use self::str::*;

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

pub fn parse_bool(s: &str) -> anyhow::Result<bool> {
    match s {
        "1" | "t" | "T" | "true" | "TRUE" | "True" => Ok(true),
        "0" | "f" | "F" | "false" | "FALSE" | "False" => Ok(false),
        _ => Err(anyhow::anyhow!("provided string was not a boolean string")),
    }
}

pub fn parse_bool_ext(s: &str) -> anyhow::Result<bool> {
    match s {
        "on" | "ON" | "On" | "enabled" => Ok(true),
        "off" | "OFF" | "Off" | "disabled" => Ok(false),
        _ => parse_bool(s),
    }
}
