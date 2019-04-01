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

use serde::Serialize;

use super::super::common::ApiVersion;
use super::super::session::{RequestBuilderExt, ServiceType, Session};
use super::super::utils::{self, ResultExt};
use super::super::Result;
use super::protocol;

/// Service type of Image API ImageService.
#[derive(Copy, Clone, Debug)]
pub struct ImageService;

impl ServiceType for ImageService {
    fn catalog_type() -> &'static str {
        "image"
    }

    fn major_version_supported(version: ApiVersion) -> bool {
        version.0 == 2
    }
}

/// Get an image.
pub fn get_image<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<protocol::Image> {
    let s = id_or_name.as_ref();
    get_image_by_id(session, s).if_not_found_then(|| get_image_by_name(session, s))
}

/// Get an image by its ID.
pub fn get_image_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<protocol::Image> {
    trace!("Fetching image {}", id.as_ref());
    let image = session
        .get::<ImageService>(&["images", id.as_ref()], None)?
        .receive_json::<protocol::Image>()?;
    trace!("Received {:?}", image);
    Ok(image)
}

/// Get an image by its name.
pub fn get_image_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<protocol::Image> {
    trace!("Get image by name {}", name.as_ref());
    let items = session
        .get::<ImageService>(&["images"], None)?
        .query(&[("name", name.as_ref())])
        .receive_json::<protocol::ImagesRoot>()?
        .images;
    let result = utils::one(
        items,
        "Image with given name or ID not found",
        "Too many images found with given name",
    )?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// List images.
pub fn list_images<Q: Serialize + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<protocol::Image>> {
    trace!("Listing images with {:?}", query);
    let result = session
        .get::<ImageService>(&["images"], None)?
        .query(query)
        .receive_json::<protocol::ImagesRoot>()?
        .images;
    trace!("Received images: {:?}", result);
    Ok(result)
}
