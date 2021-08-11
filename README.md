# hulk

Hulk is a Rust implementation of the high performance object storage.

[![build status](https://github.com/01intelligence/hulk/actions/workflows/build-and-test.yml/badge.svg?branch=master&event=push)](https://github.com/01intelligence/hulk/actions/workflows/build-and-test.yml)
[![codecov](https://codecov.io/gh/01intelligence/hulk/branch/master/graph/badge.svg?token=IPYPXRBY61)](https://codecov.io/gh/01intelligence/hulk)

## Building

### Windows

* Install Microsoft Visual Studio or Microsoft C++ Build Tools, and must install Visual Studio English Language Pack (See [#35785](https://github.com/rust-lang/rust/issues/35785)).
* Download the [Npcap SDK](https://nmap.org/npcap/), and place
  its subdirectory `Lib` in the root of this repository.
* To run, you also must have [Npcap](https://nmap.org/npcap/) installed
  (Make sure to install with the "Install Npcap in WinPcap API-compatible Mode").
