fn main() {
    println!(
        "cargo:rustc-env=HULK_BUILD_TIME={}",
        time::OffsetDateTime::now_utc().format("%Y-%m-%d %H:%M:%S"),
    );
}
