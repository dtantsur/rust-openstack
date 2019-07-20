# Change Log

## 0.3.0 (2019-07-20)

#### Breaking Changes

*   switch to osauth for session and authentication ([61d55ec6](https://github.com/dtantsur/rust-openstack/commit/61d55ec61930988d650b0dfdc64d1cc4680d94ed))

    This is a major breaking change. Starting with 0.3.0, rust-openstack no
    longer contains the authentication code. Instead, the rust-osauth crate
    is used.

    The `Session` structure has been removed in favour of the synchronous
    session from rust-osauth.

    Most of removed structures are reimported in their old locations. However,
    `RequestBuilderExt` is gone and `AuthMethod` has been renamed to `AuthType`
    to match the official Python SDK.

*   bump fallible-iterator to 0.2 and update other dependencies ([7ecf317f](https://github.com/dtantsur/rust-openstack/commit/7ecf317f0d18e27818ee47a5a7bf73b677aad416)

    The new version of fallible-iterator has slightly different public API.

## 0.2.3 (2019-02-16)

#### Features

* **compute:**  implement block device mapping (closes #76) ([19094080](https://github.com/dtantsur/rust-openstack/commit/19094080bdd08084a0c6cbe7026986f14cbeb64c))

#### Bug Fixes

*   allow inlining trivial accessors ([51a3286f](https://github.com/dtantsur/rust-openstack/commit/51a3286f4af43a0f321dfeed207d01f12572b137))
* **auth:**  do not fail when clouds.yaml contain unscoped entries ([b41666ce](https://github.com/dtantsur/rust-openstack/commit/b41666ce84fb8a9232488b6ad3554d0dd08450c4))
* **common:**  correctly parse JSON error messages (fixes #61) ([21b62c01](https://github.com/dtantsur/rust-openstack/commit/21b62c011fc52df1775d4d91b0f21d824bb82acd))

## 0.2.2 (2018-12-30)

#### Features

* **common:**  Support services without version discovery ([598ceabd](https://github.com/dtantsur/rust-openstack/commit/598ceabd179dc35171e52e82fbb67bda67d71a9b))
* **compute:**  finish creating key pairs, deprecate old names ([c88d7164](https://github.com/dtantsur/rust-openstack/commit/c88d71649173a3fb8075fe6a082035878487d194))
* **network:**  add missing Network.status() (fixes #27) ([f1dc2e28](https://github.com/dtantsur/rust-openstack/commit/f1dc2e288292a85da25fa1f7f2bc54b972543e53))

## 0.2.1 (2018-11-25)

#### Features

* **network:**
  *  updating networks (closes #50) ([d9c676de](https://github.com/dtantsur/rust-openstack/commit/d9c676de31d0bc75e6c102a7232f873c3dcb6b0a))
  *  updating subnets (closes #33) ([5e4fba5b](https://github.com/dtantsur/rust-openstack/commit/5e4fba5b14a9758d86150b4a759033dd0a249c73))

#### Bug Fixes

* **common:**
  *  only consider stable major versions ([825e371c](https://github.com/dtantsur/rust-openstack/commit/825e371ce58ec7d2c972acb8b46caa04c53878bb))
* **network:**
  *  validate and convert IDs when querying subnets and floating IPs ([87c9a57f](https://github.com/dtantsur/rust-openstack/commit/87c9a57f3ed4650e94d93525c94fa8c1e131b5e9))
  *  validate and convert network ID when querying ports ([88b61bff](https://github.com/dtantsur/rust-openstack/commit/88b61bffd0d2f52291bdbc0f92d8414ddfc2a890))

## 0.2.0 (2018-11-11)

#### Breaking Changes

* **auth:**
  * `AuthMethod::request` and `Session::request` now return `Result` with `RequestBuilder` from `reqwest` ([abed6bd7](https://github.com/dtantsur/rust-openstack/commit/abed6bd7da9a25c706dc3d5129ed39f52daf7d28))
  * `Identity` is now called `Password`, `PasswordAuth` was removed ([83dddc52](https://github.com/dtantsur/rust-openstack/commit/83dddc52d7b4f2a61a014bf9949a3237f2d85cf1))
  * `Password::new` and `NoAuth::new` now return the `Result<Error>` ([abed6bd7](https://github.com/dtantsur/rust-openstack/commit/abed6bd7da9a25c706dc3d5129ed39f52daf7d28))
* **common:**
  * The type parameter of `ResourceIterator` is now a `ResourceQuery`, not a resource ([a6c65463](https://github.com/dtantsur/rust-openstack/commit/a6c65463bd9a61c287a00945ef57fed1103e18eb))
  * `ResourceId` and `ListResources` replaced by new `ResourceQuery` ([a822aad3](https://github.com/dtantsur/rust-openstack/commit/a822aad38b69af263d2c7ae7561ff399d4d02bdb))
* **network:**
  * `Network::name()` is now `Option<String>` ([33177fc9](https://github.com/dtantsur/rust-openstack/commit/33177fc9262abe2242797ca25f08efbefca9785b))
* **session:**
  * The `ServiceInfo` structure is now private and cannot be accessed ([9742b2d5](https://github.com/dtantsur/rust-openstack/commit/9742b2d51d771e53a047b9b44d1f8efcb213458f))
  * Changed `ServiceType::api_version_headers` to `set_api_version_headers` ([abed6bd7](https://github.com/dtantsur/rust-openstack/commit/abed6bd7da9a25c706dc3d5129ed39f52daf7d28))
  * `Session.get_service_info` is replaced by more specific methods on `Session` ([9742b2d5](https://github.com/dtantsur/rust-openstack/commit/9742b2d51d771e53a047b9b44d1f8efcb213458f))

#### Features

*   update to reqwest 0.9 (some breaking changes) ([abed6bd7](https://github.com/dtantsur/rust-openstack/commit/abed6bd7da9a25c706dc3d5129ed39f52daf7d28))
* **auth:**
  *  simplify password authentication (fixes #8) ([83dddc52](https://github.com/dtantsur/rust-openstack/commit/83dddc52d7b4f2a61a014bf9949a3237f2d85cf1))
  *  support clouds.yaml ([ddda7bbb](https://github.com/dtantsur/rust-openstack/commit/ddda7bbbc6312246c85e6b6f4eead253a6722137))
* **common:**
  *  support for several major versions (first bits) ([80fb53c5](https://github.com/dtantsur/rust-openstack/commit/80fb53c58ba7ba5363fb6ee6bf0d906a662d7b80))
  *  derive Hash for ApiVersion and ErrorKind, Clone for Error (fixes #3) ([fbc9ac27](https://github.com/dtantsur/rust-openstack/commit/fbc9ac27d8deb5fdb23203a5cb8c19b988c6db12))
* **compute:**
  *  implement extra_specs and description for flavors ([48ed83cf](https://github.com/dtantsur/rust-openstack/commit/48ed83cfcbbcbc62cd0a82a2fd2c7f2ea2f64623))
  *  more server fields ([f439690f](https://github.com/dtantsur/rust-openstack/commit/f439690f991645fbc0bf88fd7794b5e2517344cf))
* **network:**
  *  creating subnets (#33) ([86c3ce3e](https://github.com/dtantsur/rust-openstack/commit/86c3ce3e5d5b7b6c3ed15b0b67ca66edbb161618))
  *  creating and deleting networks (#50) ([3e66df1e](https://github.com/dtantsur/rust-openstack/commit/3e66df1e1df3f0d2711a40a2fcf059281685876c))
  *  updating floating IPs (fixes #26) ([f9daad98](https://github.com/dtantsur/rust-openstack/commit/f9daad98a5d6d30fab904ec0868f45deee39fff4))
  *  creating floating IPs (#26) ([1fd3f1a9](https://github.com/dtantsur/rust-openstack/commit/1fd3f1a94e8128549ab1613edf714e8301ceabf1))
  *  getting, listing, deleting floating IPs (#26) ([5c214806](https://github.com/dtantsur/rust-openstack/commit/5c21480683e03f113c628b3124f0f6d3953234d0))
  *  updating ports ([2a7009b7](https://github.com/dtantsur/rust-openstack/commit/2a7009b77b446acea3b0188def934145bcd22305))
  *  accept fixed IP on port creation (fixes #28) ([dd9d361a](https://github.com/dtantsur/rust-openstack/commit/dd9d361a216fc557e2f94c644421b3364811afb9))
  *  rectify fixed IPs support in ports ([cb552ae7](https://github.com/dtantsur/rust-openstack/commit/cb552ae768d551baf8e155f9df2de67c1247d6d8))
  *  listing, getting, deleting subnets (#33) ([608448d2](https://github.com/dtantsur/rust-openstack/commit/608448d28e521bcbc2d83d00fc527497bc596bdb))
  *  creating and deleting ports (#28) ([01916711](https://github.com/dtantsur/rust-openstack/commit/019167113f4b115219a10bf59e90f1c6b70a6761))
  *  getting and listing ports (#28) ([f8f6fc58](https://github.com/dtantsur/rust-openstack/commit/f8f6fc587df5f56814e82bb037c6de02ccd046be))
* **session:**  stop exposing ServiceInfo ([9742b2d5](https://github.com/dtantsur/rust-openstack/commit/9742b2d51d771e53a047b9b44d1f8efcb213458f))

#### Bug Fixes

*   do not lookup by name in Refresh implementations ([5ecf0d44](https://github.com/dtantsur/rust-openstack/commit/5ecf0d447072404a0d06e6ece4ecd72dbad1f72c))
* **auth:**  avoid deprecated std::env::home_dir ([be49a3e0](https://github.com/dtantsur/rust-openstack/commit/be49a3e0425cf357319b32755202afea090c8ce7))
* **common:**  hide ResourceId and ListResources ([a822aad3](https://github.com/dtantsur/rust-openstack/commit/a822aad38b69af263d2c7ae7561ff399d4d02bdb))
* **compute:**  verify port supplied to NewServer ([13962afb](https://github.com/dtantsur/rust-openstack/commit/13962afbc68f5f2387aefa0b824611b3aba1d9d1))
* **network:**  network names are optional, change to Option<String> ([33177fc9](https://github.com/dtantsur/rust-openstack/commit/33177fc9262abe2242797ca25f08efbefca9785b))
