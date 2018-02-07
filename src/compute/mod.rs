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

//! Compute API implementation bits.
//!
//! # Examples
//!
//! ```rust,no_run
//! use openstack;
//!
//! let auth = openstack::auth::from_env().expect("Unable to authenticate");
//! let os = openstack::Cloud::new(auth);
//!
//! let server = os.get_server("8a1c355b-2e1e-440a-8aa8-f272df72bc32")
//!     .expect("Unable to get a server");
//! ```

mod servers;
mod v2;

pub use self::v2::V2 as ServiceType;
pub use self::v2::protocol::{AddressType, ServerAddress, ServerSortKey,
                             ServerStatus};
pub use self::servers::{Server, ServerQuery, ServerSummary};
