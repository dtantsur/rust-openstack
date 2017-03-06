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

//! Authentication modules.
//!
//! Usually, accessing OpenStack services requires authentication. This module
//! provides a way to authenticate against an Identity service, as well as
//! simple authentication implementations for standalone use.
//! The resulting objects can then be passed to the
//! [Session constructor](../session/struct.Session.html#method.new).
//!
//! See [identity module](identity/index.html) for more details on how to use
//! authentication agaist an Identity service.
//!
//! # Examples
//!
//! Creating an authentication method using projet-scoped tokens:
//!
//! ```rust,no_run
//! use openstack::auth::Identity;
//! use openstack::Session;
//!
//! let auth = Identity::new("https://my.cloud.com/identity").unwrap()
//!     .with_user("admin", "pa$$w0rd", "My Domain")
//!     .with_project_scope("project1", "My Domain")
//!     .create().expect("Failed to authenticate");
//! let session = Session::new(auth);
//! ```
//!
//! Creating an authentication method from environment variables:
//!
//! ```rust,no_run
//! use openstack::auth::Identity;
//! use openstack::Session;
//!
//! let auth = Identity::from_env().expect("Failed to authenticate");
//! let session = Session::new(auth);
//! ```
//!
//! Creating a dummy authentication method for use against clouds that do not
//! have actual authentication:
//!
//! ```
//! use openstack::auth::NoAuth;
//! use openstack::Session;
//!
//! let auth = NoAuth::new("https://my.cloud.com/some-service").unwrap();
//! let session = Session::new(auth);
//! ```

mod base;
pub mod identity;
mod simple;

pub use self::base::{Method, Token};
pub use self::simple::{SimpleToken, NoAuth};
pub use self::identity::Identity;
