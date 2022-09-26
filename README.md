OpenStack SDK for Rust
======================

![CI](https://github.com/dtantsur/rust-openstack/workflows/CI/badge.svg)
[![License](https://img.shields.io/crates/l/openstack.svg)](https://github.com/dtantsur/rust-openstack/blob/master/LICENSE)
[![Latest
Version](https://img.shields.io/crates/v/openstack.svg)](https://crates.io/crates/openstack)
[![Documentation](https://img.shields.io/badge/documentation-latest-blueviolet.svg)](https://docs.rs/openstack)

The goal of this project is to provide a simple API for working with OpenStack
clouds. It is still work-in-progress, some bits are not implemented.

## Usage

Use standard [cargo](http://crates.io) tool to build and test. Add a dependency
on `openstack` crate to your software to use this library.

### Examples

There is an example that lists all running servers (their ID and name).
Source your Keystone V3 credentials and run:

    cargo run --example list-servers

Enable verbose logging by using standard `RUST_LOG` variable:

    RUST_LOG=openstack cargo run --example list-servers
