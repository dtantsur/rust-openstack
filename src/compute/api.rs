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

//! Low-level bits exposing the Compute API.

use super::super::service::{ServiceApi, ServiceType};

/// Service type of Compute API V2.
#[derive(Copy, Clone, Debug)]
pub struct ComputeV2Type;

/// Low-level service API implementation.
pub type ComputeV2<'session, Auth> = ServiceApi<'session, Auth, ComputeV2Type>;

const SERVICE_TYPE: &'static str = "compute";
const SUFFIX: &'static str = "v2.1";

impl ServiceType for ComputeV2Type {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn version_suffix() -> Option<&'static str> {
        Some(SUFFIX)
    }
}
