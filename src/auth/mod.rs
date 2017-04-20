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
//!
//! The usual workflow for connecting to OpenStack API is as follows:
//!
//! 1. Create a suitable authentication method.
//! 2. Populate it with authentication data (credentials, etc).
//! 3. Create a [Session](../session/struct.Session.html) by using the
//!    [Session constructor](../session/struct.Session.html#method.new).
//! 4. Pass a reference to the resulting session to various API managers.
//!
//! # Using password authentication
//!
//! Start with creating an [Identity](struct.Identity.html) object which will
//! guide you through setting all necessary values.
//! [PasswordAuth](struct.PasswordAuth.html) is the actual implementation
//! of the authentication [method](trait.AuthMethod.html) trait.
//!
//! Note that as of now, only project-scoped tokens are supported.
//! An attempt to create unscoped tokens always fails. This restriction may
//! be lifted in the future.
//!
//! # Examples
//!
//! Creating an authentication method using project-scoped tokens:
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
//!
//! # Limitations
//!
//! * Only Identity API v3 is supported and planned for support.

mod base;
mod identity;
mod simple;

pub use self::base::{AuthMethod, BoxedClone};
pub use self::simple::NoAuth;
pub use self::identity::{Identity, PasswordAuth};
