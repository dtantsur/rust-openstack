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

use reqwest::Method;
use serde::Serialize;

use super::super::Result;
use super::super::common::ApiVersion;
use super::super::session::{Session, ServiceType};
use super::super::utils::{self, ResultExt};
use super::protocol;


/// Extensions for Session.
pub trait V2API {
    /// Get an image.
    fn get_image<S: AsRef<str>>(&self, id_or_name: S) -> Result<protocol::Image> {
        let s = id_or_name.as_ref();
        self.get_image_by_id(s).if_not_found_then(|| self.get_image_by_name(s))
    }

    /// Get an image by its ID.
    fn get_image_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Image>;

    /// Get an image by its name.
    fn get_image_by_name<S: AsRef<str>>(&self, id: S) -> Result<protocol::Image>;

    /// List images.
    fn list_images<Q: Serialize + Debug>(&self, query: &Q)
        -> Result<Vec<protocol::Image>>;
}


/// Service type of Image API V2.
#[derive(Copy, Clone, Debug)]
pub struct V2;


const SERVICE_TYPE: &str = "image";


impl V2API for Session {
    fn get_image_by_id<S: AsRef<str>>(&self, id: S) -> Result<protocol::Image> {
        trace!("Fetching image {}", id.as_ref());
        let image = self.request::<V2>(Method::Get,
                                       &["images", id.as_ref()],
                                       None)?
           .receive_json::<protocol::Image>()?;
        trace!("Received {:?}", image);
        Ok(image)
    }

    fn get_image_by_name<S: AsRef<str>>(&self, name: S) -> Result<protocol::Image> {
        trace!("Get image by name {}", name.as_ref());
        let items = self.request::<V2>(Method::Get, &["images"], None)?
            .query(&[("name", name.as_ref())])
            .receive_json::<protocol::ImagesRoot>()?.images;
        let result = utils::one(items, "Image with given name or ID not found",
                                "Too many images found with given name")?;
        trace!("Received {:?}", result);
        Ok(result)
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

    fn major_version_supported(version: ApiVersion) -> bool {
        version.0 == 2
    }
}
