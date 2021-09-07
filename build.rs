fn main() {
    println!(
        "cargo:rustc-env=HULK_BUILD_TIME={}",
        time::OffsetDateTime::now_utc().format("%Y-%m-%d %H:%M:%S"),
    );

    tonic_build::configure()
        .out_dir("src/proto")
        .compile(
            &[
                "proto/common.proto",
                "proto/peer.proto",
                "proto/storage.proto",
            ],
            &["proto"],
        )
        .unwrap();

    print_link_search_path();
}

fn print_link_search_path() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "x86_64" {
            println!("cargo:rustc-link-search=native={}/lib/x64", manifest_dir);
        } else {
            println!("cargo:rustc-link-search=native={}/lib", manifest_dir);
        }
    }
}
