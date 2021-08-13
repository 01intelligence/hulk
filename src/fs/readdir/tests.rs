extern crate test;
use std::collections::HashMap;
use std::path::PathBuf;

use test::Bencher;

use super::*;
use crate::fs;
use crate::utils::assert::*;
use crate::utils::{self, Rng};

async fn generate_dir(n: usize) -> (tempfile::TempDir, HashMap<PathBuf, fs::File>) {
    let dir = tempfile::tempdir_in(".").unwrap();
    let mut files = HashMap::new();
    for _ in 0..n {
        let rnd = utils::rng_seed_now().gen::<[u8; 8]>();
        let tmp_file = format!("test-readdir-{}.tmp", hex::encode(rnd));
        let tmp_file = dir.path().join(tmp_file);
        let file = assert_ok!(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(tmp_file.as_path())
                .await
        );
        files.insert(assert_ok!(tmp_file.canonicalize()), file);
    }
    (dir, files)
}

#[tokio::test]
async fn test_readdir() {
    let mut n = 10;
    let (dir, files) = generate_dir(n).await;
    let mut read_stream = assert_ok!(read_dir(dir.path()).await);
    while let Some(dir) = assert_ok!(read_stream.next_entry().await) {
        let got = assert_ok!(dir.path().canonicalize());
        assert!(files.contains_key(&got));
        n -= 1;
    }
    assert_eq!(n, 0);
}

fn bench_self_readdir(b: &mut Bencher, n: usize) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()
        .unwrap();
    let (dir, files) = runtime.block_on(async { generate_dir(n).await });
    b.iter(|| {
        runtime.block_on(async {
            let mut n = n;
            let mut read_stream = assert_ok!(read_dir(dir.path()).await);
            while let Some(_) = assert_ok!(read_stream.next_entry().await) {
                n -= 1;
            }
            assert_eq!(n, 0);
        });
    })
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
fn bench_tokio_readdir(b: &mut Bencher, n: usize) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()
        .unwrap();
    let (dir, files) = runtime.block_on(async { generate_dir(n).await });
    b.iter(|| {
        runtime.block_on(async {
            let mut n = n;
            let mut read_stream = assert_ok!(tokio::fs::read_dir(dir.path()).await);
            while let Some(_) = assert_ok!(read_stream.next_entry().await) {
                n -= 1;
            }
            assert_eq!(n, 0);
        });
    })
}

#[bench]
fn bench_self_readdir_10k(b: &mut Bencher) {
    let n = 10000;
    bench_self_readdir(b, n);
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
#[bench]
fn bench_tokio_readdir_10k(b: &mut Bencher) {
    let n = 10000;
    bench_tokio_readdir(b, n);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn bench_self_readdir_1m() {
    test::bench::run_once(|b| {
        let n = 1000000;
        bench_self_readdir(b, n);
    });
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn bench_tokio_readdir_1m() {
    test::bench::run_once(|b| {
        let n = 1000000;
        bench_tokio_readdir(b, n);
    });
}
