Contributing
============

## Testing and CI

**rust-openstack** ships with both unit and integration tests. The unit tests
are in the modules they're testing, the integration tests are in the [tests
directory](https://github.com/dtantsur/rust-openstack/tree/master/tests).

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

## Code structure

Internally, **rust-openstack** consists of three large parts: pluggable
authentication, internal low-level API and public high-level API.

### Pluggable authentication

The authentication code is contained in the [auth
module](https://docs.rs/openstack/latest/openstack/auth/index.html). Each
authentication method must in the end yield a structure implementing the
[AuthMethod trait](https://docs.rs/openstack/latest/openstack/auth/trait.AuthMethod.html).

### Low-level API

The low-level API is represented by
[Session](https://docs.rs/openstack/latest/openstack/session/struct.Session.html).
It owns the authentication method and provides function to make authenticated
HTTP calls, similar to the Python [keystoneauth
library](https://docs.openstack.org/keystoneauth/latest/).

Each service provides a (private) structure implementing the [ServiceType
trait](https://docs.rs/openstack/latest/openstack/session/trait.ServiceType.html).
Its main goal is to introspect the API and return a properly populated
[ServiceInfo structure](https://docs.rs/openstack/latest/openstack/session/struct.ServiceInfo.html).
Many of the Session methods use something implementing ServiceType as a type
parameter.

The actual service API calls are implemented via a private extension trait
for Session. They should stay as close to the underlying HTTP as possible.
They should accept and return either simple values or structures directly
mapping to the input or output of the corresponding API and
serializable/deserializable with the [serde library](https://serde.rs/).

As an example, see [Compute protocol
structures](https://github.com/dtantsur/rust-openstack/blob/master/src/compute/protocol.rs)
and the [Compute extension
trait](https://github.com/dtantsur/rust-openstack/blob/master/src/compute/base.rs).
