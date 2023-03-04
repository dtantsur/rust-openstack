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

use osauth::services::IMAGE;
use osauth::ErrorKind;
use serde::Serialize;

use super::super::session::Session;
use super::super::utils;
use super::super::Result;
use super::protocol::*;

/// Get an image.
pub async fn get_image<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Image> {
    let s = id_or_name.as_ref();
    match get_image_by_id(session, s).await {
        Ok(value) => Ok(value),
        Err(err) if err.kind() == ErrorKind::ResourceNotFound => {
            get_image_by_name(session, s).await
        }
        Err(err) => Err(err),
    }
}

/// Get an image by its ID.
pub async fn get_image_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<Image> {
    trace!("Fetching image {}", id.as_ref());
    let image: Image = session.get_json(IMAGE, &["images", id.as_ref()]).await?;
    trace!("Received {:?}", image);
    Ok(image)
}

/// Get an image by its name.
pub async fn get_image_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Image> {
    trace!("Get image by name {}", name.as_ref());
    let root: ImagesRoot = session
        .get(IMAGE, &["images"])
        .query(&[("name", name.as_ref())])
        .fetch()
        .await?;
    let result = utils::one(
        root.images,
        "Image with given name or ID not found",
        "Too many images found with given name",
    )?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// List images.
pub async fn list_images<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<Image>> {
    trace!("Listing images with {:?}", query);
    let root: ImagesRoot = session.get(IMAGE, &["images"]).query(query).fetch().await?;
    trace!("Received images: {:?}", root.images);
    Ok(root.images)
}
