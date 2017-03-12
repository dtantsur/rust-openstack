OpenStack SDK for Rust
======================

[![Build
Status](https://travis-ci.org/dtantsur/rust-openstack.svg?branch=master)](https://travis-ci.org/dtantsur/rust-openstack)
[![License](https://img.shields.io/crates/l/openstack.svg)](https://github.com/dtantsur/rust-openstack/blob/master/LICENSE)
[![Latest
Version](https://img.shields.io/crates/v/openstack.svg)](https://crates.io/crates/openstack)

The goal of this project is to provide a simple API for working with OpenStack
clouds. This is an early work-in-progress, don't expect too much of it.

Documentation: [master](https://dtantsur.github.io/rust-openstack/openstack/),
[latest release](https://docs.rs/openstack/).

## Usage

Use standard [cargo](http://crates.io) tool to build and test. Add a dependency
on `openstack` crate to your software to use this library.

### Examples

There is an example retrieving a token from Keystone. Source your OpenStack
credentials and run from the project root:

    cargo run --example get-token

Another example lists all running servers (their ID and name):

    cargo run --example list-servers

Enable verbose logging by using standard `RUST_LOG` variable:

    RUST_LOG=openstack cargo run --example list-servers
