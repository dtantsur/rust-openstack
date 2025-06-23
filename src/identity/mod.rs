// Copyright 2019-2020 Dmitry Tantsur <dtantsur@protonmail.com>
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

//! Authentication using Identity API v3.
//!
//! Currently supports [Password](struct.Password.html) and [ApplicationCredential] authentication.
//! Identity API v2 is not and will not be supported.

mod application_credential;
mod catalog;
mod internal;
mod password;
pub(crate) mod protocol;
mod token;

use super::IdOrName;

pub use self::application_credential::ApplicationCredential;
pub use self::password::Password;
pub use self::token::Token;

const MISSING_SUBJECT_HEADER: &str = "Missing X-Subject-Token header";
const INVALID_SUBJECT_HEADER: &str = "Invalid X-Subject-Token header";
// Required validity time in minutes. Here we refresh the token if it expires
// in 10 minutes or less.
const TOKEN_MIN_VALIDITY: i64 = 10;

/// A scope of a token.
///
/// Only project scopes are currently supported.
#[derive(Debug)]
pub enum Scope {
    /// A token scoped to a project.
    Project {
        /// Project ID or name.
        project: IdOrName,
        /// ID or name of the project domain.
        domain: Option<IdOrName>,
    },
}
