// Copyright 2019 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Foundation bits exposing the object storage API.

use futures::io::AsyncRead;
use futures::Stream;
use osauth::request::{self, NO_PATH};
use osauth::services::OBJECT_STORAGE;
use reqwest::{Method, StatusCode};

use super::super::session::Session;
use super::super::utils::Query;
use super::super::Result;
use super::protocol::*;
use super::utils::{async_read_to_body, body_to_async_read};

/// Create a new container.
///
/// Returns `true` if the container was created, `false` if it existed.
pub async fn create_container<C>(session: &Session, container: C) -> Result<bool>
where
    C: AsRef<str>,
{
    let c_id = container.as_ref();
    debug!("Creating container {}", c_id);
    let resp = session.put_empty(OBJECT_STORAGE, &[c_id], None).await?;
    let result = resp.status() == StatusCode::CREATED;
    if result {
        debug!("Successfully created container {}", c_id);
    } else {
        debug!("Container {} already exists", c_id);
    }
    Ok(result)
}

/// Create a new object.
pub async fn create_object<C, O, R>(
    session: &Session,
    container: C,
    object: O,
    body: R,
) -> Result<Object>
where
    C: AsRef<str>,
    O: AsRef<str>,
    R: AsyncRead + Send + Sync + 'static,
{
    let c_id = container.as_ref();
    let o_id = object.as_ref();
    debug!("Creating object {} in container {}", o_id, c_id);
    let _ = request::send_checked(
        session
            .request(OBJECT_STORAGE, Method::PUT, &[c_id, o_id], None)
            .await?
            .body(async_read_to_body(body)),
    )
    .await?;
    debug!("Successfully created object {} in container {}", o_id, c_id);
    // We need to retrieve the size, issue HEAD.
    get_object(session, c_id, o_id).await
}

/// Delete an empty container.
pub async fn delete_container<C>(session: &Session, container: C) -> Result<()>
where
    C: AsRef<str>,
{
    let c_id = container.as_ref();
    debug!("Deleting container {}", c_id);
    let _ = session.delete(OBJECT_STORAGE, &[c_id], None).await?;
    debug!("Successfully deleted container {}", c_id);
    Ok(())
}

/// Delete an object.
pub async fn delete_object<C, O>(session: &Session, container: C, object: O) -> Result<()>
where
    C: AsRef<str>,
    O: AsRef<str>,
{
    let c_id = container.as_ref();
    let o_id = object.as_ref();
    debug!("Deleting object {} in container {}", o_id, c_id);
    let _ = session.delete(OBJECT_STORAGE, &[c_id, o_id], None).await?;
    debug!("Successfully deleted object {} in container {}", o_id, c_id);
    Ok(())
}

/// Get container metadata.
pub async fn get_container<C>(session: &Session, container: C) -> Result<Container>
where
    C: AsRef<str>,
{
    let c_id = container.as_ref();
    trace!("Requesting container {}", c_id);
    let resp = request::send_checked(
        session
            .request(OBJECT_STORAGE, Method::HEAD, &[c_id], None)
            .await?,
    )
    .await?;
    let result = Container::from_headers(c_id, resp.headers())?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// Get object metadata.
pub async fn get_object<C, O>(session: &Session, container: C, object: O) -> Result<Object>
where
    C: AsRef<str>,
    O: AsRef<str>,
{
    let c_id = container.as_ref();
    let o_id = object.as_ref();
    trace!("Requesting object {} from container {}", o_id, c_id);
    let resp = request::send_checked(
        session
            .request(OBJECT_STORAGE, Method::HEAD, &[c_id, o_id], None)
            .await?,
    )
    .await?;
    let result = Object::from_headers(o_id, resp.headers())?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// Download the requested object.
pub async fn download_object<C, O>(
    session: &Session,
    container: C,
    object: O,
) -> Result<impl AsyncRead + Send + 'static>
where
    C: AsRef<str>,
    O: AsRef<str>,
{
    let c_id = container.as_ref();
    let o_id = object.as_ref();
    trace!("Downloading object {} from container {}", o_id, c_id);
    let resp = session.get(OBJECT_STORAGE, &[c_id, o_id], None).await?;
    Ok(body_to_async_read(resp))
}

/// List containers for the current account.
pub async fn list_containers(
    session: &Session,
    mut query: Query,
    limit: Option<usize>,
    marker: Option<String>,
) -> Result<impl Stream<Item = Result<Container>>> {
    query.push_str("format", "json");
    trace!("Listing containers with {:?}", query);
    session
        .get_json_query_paginated(OBJECT_STORAGE, NO_PATH, query, None, limit, marker)
        .await
}

/// List objects in a given container.
pub async fn list_objects<C>(
    session: &Session,
    container: C,
    mut query: Query,
    limit: Option<usize>,
    marker: Option<String>,
) -> Result<impl Stream<Item = Result<Object>>>
where
    C: AsRef<str> + 'static,
{
    query.push_str("format", "json");
    let id = container.as_ref();
    trace!("Listing objects in container {} with {:?}", id, query);
    session
        .get_json_query_paginated(OBJECT_STORAGE, &[id], query, None, limit, marker)
        .await
}
