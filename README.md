OpenStack client for Rust
=========================

[![Build
Status](https://travis-ci.org/dtantsur/rust-openstack.svg?branch=master)](https://travis-ci.org/dtantsur/rust-openstack)
[![License](https://img.shields.io/crates/l/openstack.svg)](https://github.com/dtantsur/rust-openstack/blob/master/LICENSE)
[![Latest
Version](https://img.shields.io/crates/v/openstack.svg)](https://crates.io/crates/openstack)

The goal of this project is to provide a simple API for working with OpenStack
clouds. This is an early work-in-progress, don't expect too much of it.

Limitations
-----------

* Only Identity API v3 is supported and planned for support.

Build
-----

Use standard [cargo](http://crates.io) tool to build and test.

Demo
----

There is an example retrieving a token from Keystone. Source your OpenStack
credentials and run:

    cargo run --example get-token

Docs
----

... are not hosted anywhere so far, so build them yourself:

    cargo doc

and point your browser at `target/doc/openstack/index.html`.
