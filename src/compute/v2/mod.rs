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

//! Compute API (v2 with microversions) implementation.
//!
//! Currently supported functionality:
//!
//! * [server management](struct.ServerManager.html) (incomplete)
//!
//! # Examples
//!
//! ```rust,no_run
//! use openstack;
//!
//! let auth = openstack::auth::Identity::from_env()
//!     .expect("Unable to authenticate");
//! let session = openstack::Session::new(auth);
//! let servers = openstack::compute::v2::servers(&session);
//!
//! let server = servers.get("8a1c355b-2e1e-440a-8aa8-f272df72bc32")
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
//! let servers = openstack::compute::v2::servers(&session);
//! ```

mod base;
mod servermanager;
mod protocol;

pub use self::base::V2;
pub use self::servermanager::{servers, Server, ServerList, ServerManager,
                              ServerQuery, ServerSummary};
