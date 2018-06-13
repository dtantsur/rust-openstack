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

### High-level API

The [Cloud structure](https://docs.rs/openstack/latest/openstack/struct.Cloud.html)
is an entry point to the high-level API. This is what the consumers of
**rust-openstack** are supposed to use. Its methods follow the following
patterns:

* The `get` methods return an object by its ID (if applicable) or name (if
  applicable). They should return an error of kind
  [TooManyItems](https://docs.rs/openstack/latest/openstack/enum.ErrorKind.html#variant.TooManyItems)
  if e.g. the name is not unique.

* The `list` methods return a `Vec` with all the objects of the given kind.
  They are expected to handle pagination internally.

* The `find` methods do not yield results immediately. Instead, they return
  a builder object, with which the user can construct a query. Such object,
  in turn, should provide at least the following methods:

    - `all` returns all objects matching the query in a `Vec`.

    - `one` returns one and exactly one object matching the query, failing if
      the query yield nothing or more than one results.

    - `into_iter` consumes the query object, returning a
      [ResourceIterator](https://docs.rs/openstack/latest/openstack/common/struct.ResourceIterator.html).

    - It should also implement `IntoFallibleIterator` from the
      [fallible-iterator crate](https://crates.io/crates/fallible-iterator).

* The `new` methods start creating a new resource. Similar to `find` methods,
  they don't do anything immediately, but rather return a builder object that
  allows the caller to populate the resource's field. The `new` methods should
  only take mandatory field as direct arguments. Builder objects should at
  least have a `create` method that starts the creation process and returns
  a [waiter](https://crates.io/crates/waiter) object.

The resource structures returned by these methods must abstract away the
underlying protocol details, especially microversions. Care has to be taken
to wrap attributes that are not available in all versions of the protocol
in `Option`. Actual API actions should be delegated to the low-level API.
