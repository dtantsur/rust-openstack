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

use std::fmt::Debug;

use reqwest::{Method, Url};
use serde::Serialize;

use super::super::Result;
use super::super::auth::AuthMethod;
use super::super::common;
use super::super::service::{ServiceInfo, ServiceType};
use super::super::session::Session;
use super::protocol;


/// Extensions for Session.
pub trait V2API {
    /// Get a image.
    fn get_image<S: AsRef<str>>(&self, id: S) -> Result<protocol::Image>;

    /// List images.
    fn list_images<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Image>>;
}


/// Service type of Image API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &'static str = "image";
// FIXME(dtantsur): detect versions instead of hardcoding Kilo.
const VERSION_ID: &'static str = "v2.3";


impl V2API for Session {
    fn get_image<S: AsRef<str>>(&self, id: S) -> Result<protocol::Image> {
        trace!("Fetching image {}", id.as_ref());
        let image = self.request::<V2>(Method::Get,
                                       &["images", id.as_ref()],
                                       None)?
           .receive_json::<protocol::Image>()?;
        trace!("Received {:?}", image);
        Ok(image)
    }

    fn list_images<Q: Serialize + Debug>(&self, query: &Q)
            -> Result<Vec<protocol::Image>> {
        trace!("Listing images with {:?}", query);
        let result = self.request::<V2>(Method::Get, &["images"], None)?
           .query(query).receive_json::<protocol::ImagesRoot>()?.images;
        trace!("Received images: {:?}", result);
        Ok(result)
    }
}


impl ServiceType for V2 {
    fn catalog_type() -> &'static str {
        SERVICE_TYPE
    }

    fn service_info(endpoint: Url, auth: &AuthMethod) -> Result<ServiceInfo> {
        common::fetch_service_info(endpoint, auth, SERVICE_TYPE, VERSION_ID)
    }
}
