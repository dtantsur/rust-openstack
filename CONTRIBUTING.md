Contributing
============

# Testing and CI

**rust-openstack** ships with both unit and integration tests. The unit tests
are in the modules they're testing, the integration tests are in the
[tests directory](https://github.com/dtantsur/rust-openstack/tree/master/tests).

To run only unit and doc tests use:

    # Run unit tests with all services enabled
    cargo test --lib
    # Run doc tests
    cargo test --doc
    # Run unit tests with all services disabled
    cargo test --no-default-features --lib

Sometimes enabling full logging first is helpful:

    export RUST_BACKTRACE=1
    export RUST_LOG=openstack

To run all tests including integration ones:

    export RUST_OPENSTACK_FLAVOR=<flavor>
    export RUST_OPENSTACK_NETWORK=<network>
    export RUST_OPENSTACK_IMAGE=<image>
    export RUST_OPENSTACK_KEYPAIR=<ssh key file>

    cargo test -- --test-threads=1

The last command is run in the CI on every pull requests - look for comments
from [theopenlab-ci user](https://github.com/apps/theopenlab-ci).
