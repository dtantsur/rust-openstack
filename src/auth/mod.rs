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
//! 3. Create a [Cloud](../struct.Cloud.html).
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
//! use openstack;
//!
//! let auth = openstack::auth::Identity::new("https://my.cloud.com/identity").unwrap()
//!     .with_user("admin", "pa$$w0rd", "My Domain")
//!     .with_project_scope("project1", "My Domain")
//!     .create().expect("Failed to authenticate");
//! let os = openstack::Cloud::new(auth);
//! ```
//!
//! Creating an authentication method from environment variables:
//!
//! ```rust,no_run
//! use openstack;
//!
//! let auth = openstack::auth::from_env().expect("Failed to authenticate");
//! let os = openstack::Cloud::new(auth);
//! ```
//!
//! Creating a dummy authentication method for use against clouds that do not
//! have actual authentication:
//!
//! ```
//! use openstack;
//!
//! let auth = openstack::auth::NoAuth::new("https://my.cloud.com/some-service")
//!     .unwrap();
//! let os = openstack::Cloud::new(auth);
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

use std::env;

use super::ApiResult;
use super::ApiError::InvalidInput;

const MISSING_ENV_VARS: &'static str =
    "Not all required environment variables were provided";
const INVALID_ENV_AUTH_URL: &'static str =
    "Malformed authentication URL provided in the environment";

#[inline]
fn _get_env(name: &str) -> ApiResult<String> {
    env::var(name).or(Err(InvalidInput(String::from(MISSING_ENV_VARS))))
}


/// Create an authentication method from environment variables.
pub fn from_env() -> ApiResult<PasswordAuth> {
    let auth_url = _get_env("OS_AUTH_URL")?;
    let id = Identity::new(&auth_url).map_err(|_| {
        InvalidInput(String::from(INVALID_ENV_AUTH_URL))
    })?;

    let user_name = _get_env("OS_USERNAME")?;
    let password = _get_env("OS_PASSWORD")?;
    let project_name = _get_env("OS_PROJECT_NAME")?;

    let user_domain = env::var("OS_USER_DOMAIN_NAME")
        .unwrap_or(String::from("Default"));
    let project_domain = env::var("OS_PROJECT_DOMAIN_NAME")
        .unwrap_or(String::from("Default"));

    id.with_user(user_name, password, user_domain)
        .with_project_scope(project_name, project_domain)
        .create()
}
