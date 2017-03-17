// Copyright 2017 Dmitry Tantsur <divius.inside@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Compute API support.
//!
//! Start with creating an [API v2](api_v2/struct.V2Api.html) instance via
//! its [new method](api_v2/struct.V2Api.html#method.new) or via handy
//! [v2 function](fn.v2.html). The resulting object get create specific
//! API managers for working with different parts of API, e.g.
//! [ServerManager](api_v2/servers/struct.ServerManager.html).
//!
//! Currently supported functionality:
//!
//! * [server management](api_v2/servers/index.html) (incomplete)
//!
//! # Examples
//!
//! ```rust,no_run
//! use openstack;
//!
//! let auth = openstack::auth::Identity::from_env()
//!     .expect("Unable to authenticate");
//! let session = openstack::Session::new(auth);
//! let compute = openstack::compute::v2(&session);
//!
//! let server_list = compute.servers().list()
//!     .fetch().expect("Unable to fetch servers");
//! let one_server = compute.servers()
//!     .get("8a1c355b-2e1e-440a-8aa8-f272df72bc32")
//!     .expect("Unable to get a server");
//! ```
//!
//! Compute API supports version negotiation:
//!
//! ```rust,no_run
//! use openstack;
//!
//! let auth = openstack::auth::Identity::from_env()
//!     .expect("Unable to authenticate");
//! let mut session = openstack::Session::new(auth);
//! let version = session.negotiate_api_version::<openstack::compute::V2>(
//!     openstack::ApiVersionRequest::Exact(openstack::ApiVersion(2, 10))
//! ).expect("API version 2.10 is not supported");
//!
//! let compute = openstack::compute::v2(&session);
//! ```

pub mod api_v2;

pub use self::api_v2::new as v2;
pub use self::api_v2::V2ServiceType as V2;
