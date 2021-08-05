fn main() {
    println!(
        "cargo:rustc-env=HULK_BUILD_TIME={}",
        time::OffsetDateTime::now_utc().format("%Y-%m-%d %H:%M:%S"),
    );

    tonic_build::configure()
        .out_dir("src/proto")
        .compile(
            &["proto/common.proto", "proto/peer_service.proto"],
            &["proto"],
        )
        .unwrap();
}
