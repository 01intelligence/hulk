ENABLE_FEATURES ?=

BUILD_INFO_GIT_FALLBACK := "Unknown (no git or not git repo)"
BUILD_INFO_RUSTC_FALLBACK := "Unknown"
export HULK_ENABLE_FEATURES := ${ENABLE_FEATURES}
export HULK_BUILD_RUSTC_VERSION := $(shell rustc --version 2> /dev/null || echo ${BUILD_INFO_RUSTC_FALLBACK})
export HULK_BUILD_GIT_HASH ?= $(shell git rev-parse HEAD 2> /dev/null || echo ${BUILD_INFO_GIT_FALLBACK})
export HULK_BUILD_GIT_TAG ?= $(shell git describe --tag 2> /dev/null || echo ${BUILD_INFO_GIT_FALLBACK})
export HULK_BUILD_GIT_BRANCH ?= $(shell git rev-parse --abbrev-ref HEAD 2> /dev/null || echo ${BUILD_INFO_GIT_FALLBACK})

export CARGO_BUILD_PIPELINING=true

# Almost all the rules in this Makefile are PHONY
# Declaring a rule as PHONY could improve correctness
# But probably instead just improves performance by a little bit
.PHONY: clean all build build-allow-warnings test test-allow-warnings
.PHONY: bench bench-allow-warnings fmt fmt-check check
.PHONY: doc download-libs

default: build-allow-warnings

clean:
	cargo +nightly clean

all: build

build: export HULK_PROFILE=debug
build:
	cargo +nightly build --verbose --no-default-features

build-allow-warnings: export HULK_PROFILE=debug
build-allow-warnings:
	RUSTFLAGS="-A warnings" cargo +nightly build --no-default-features

test:
	cargo +nightly test --verbose --no-default-features -- --skip bench

test-allow-warnings: export HULK_PROFILE=debug
test-allow-warnings:
	RUSTFLAGS="-A warnings" cargo +nightly test --no-default-features -- --skip bench

bench:
	cargo +nightly bench --verbose --no-default-features

bench-allow-warnings: export HULK_PROFILE=debug
bench-allow-warnings:
	RUSTFLAGS="-A warnings" cargo +nightly bench --no-default-features

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

check:
	cargo +nightly clippy

doc:
	RUSTDOCFLAGS="--enable-index-page -Zunstable-options" cargo +nightly doc --no-deps

download-libs:
	curl -o npcap-sdk-1.10.zip https://nmap.org/npcap/dist/npcap-sdk-1.10.zip
	unzip -o npcap-sdk-1.10.zip Lib/**
