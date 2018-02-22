// Copyright 2018 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Foundation bits exposing the Image API.

use reqwest::Url;

use super::super::Result;
use super::super::auth::AuthMethod;
use super::super::common;
use super::super::service::{ServiceInfo, ServiceType};


/// Service type of Image API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &'static str = "image";
// FIXME(dtantsur): detect versions instead of hardcoding Kilo.
const VERSION_ID: &'static str = "v2.3";


impl ServiceType for V2 {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn service_info(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo> {
        common::fetch_service_info(endpoint, auth, SERVICE_TYPE, VERSION_ID)
    }
}
