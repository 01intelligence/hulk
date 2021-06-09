ENABLE_FEATURES ?=

BUILD_INFO_GIT_FALLBACK := "Unknown (no git or not git repo)"
BUILD_INFO_RUSTC_FALLBACK := "Unknown"
export HULK_ENABLE_FEATURES := ${ENABLE_FEATURES}
export HULK_BUILD_RUSTC_VERSION := $(shell rustc --version 2> /dev/null || echo ${BUILD_INFO_RUSTC_FALLBACK})
export HULK_BUILD_GIT_HASH ?= $(shell git rev-parse HEAD 2> /dev/null || echo ${BUILD_INFO_GIT_FALLBACK})
export HULK_BUILD_GIT_TAG ?= $(shell git describe --tag 2> /dev/null || echo ${BUILD_INFO_GIT_FALLBACK})
export HULK_BUILD_GIT_BRANCH ?= $(shell git rev-parse --abbrev-ref HEAD 2> /dev/null || echo ${BUILD_INFO_GIT_FALLBACK})

# Almost all the rules in this Makefile are PHONY
# Declaring a rule as PHONY could improve correctness
# But probably instead just improves performance by a little bit
.PHONY: clean all build check

default: build

clean:
	cargo clean

all: build

build: export HULK_PROFILE=debug
build:
	cargo build --no-default-features

check:
	cargo clippy
