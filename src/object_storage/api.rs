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

use std::io;

use futures::stream::Stream;
use osauth::request::NO_PATH;
use osauth::services::OBJECT_STORAGE;
use osauth::sync::{SyncBody, SyncStream, SyncStreamItem};
use reqwest::{Method, StatusCode};

use super::super::session::Session;
use super::super::utils::Query;
use super::super::Result;
use super::objects::ObjectHeaders;
use super::protocol::*;

/// Create a new container.
///
/// Returns `true` if the container was created, `false` if it existed.
pub fn create_container<C>(session: &Session, container: C) -> Result<bool>
where
    C: AsRef<str>,
{
    let c_id = container.as_ref();
    debug!("Creating container {}", c_id);
    let resp = session.put_empty(OBJECT_STORAGE, &[c_id], None)?;
    let result = resp.status() == StatusCode::CREATED;
    if result {
        debug!("Successfully created container {}", c_id);
    } else {
        debug!("Container {} already exists", c_id);
    }
    Ok(result)
}

/// Create a new object.
pub fn create_object<C, O, R>(
    session: &Session,
    container: C,
    object: O,
    body: R,
    headers: ObjectHeaders,
) -> Result<Object>
where
    C: AsRef<str>,
    O: AsRef<str>,
    R: io::Read + Sync + Send + 'static,
{
    let c_id = container.as_ref();
    let o_id = object.as_ref();
    debug!("Creating object {} in container {}", o_id, c_id);
    let mut req = session.request(OBJECT_STORAGE, Method::PUT, &[&c_id, &o_id], None)?;

    if let Some(delete_after) = headers.delete_after {
        req = req.header("X-Delete-After", delete_after);
    }

    if let Some(delete_at) = headers.delete_at {
        req = req.header("X-Delete-At", delete_at);
    }

    for (key, value) in headers.metadata {
        req = req.header(&format!("X-Object-Meta-{}", key), value);
    }

    let _ = session.send_checked(req.body(SyncBody::new(body)))?;
    debug!("Successfully created object {} in container {}", o_id, c_id);
    // We need to retrieve the size, issue HEAD.
    get_object(session, c_id, o_id)
}

/// Delete an empty container.
pub fn delete_container<C>(session: &Session, container: C) -> Result<()>
where
    C: AsRef<str>,
{
    let c_id = container.as_ref();
    debug!("Deleting container {}", c_id);
    let _ = session.delete(OBJECT_STORAGE, &[c_id], None)?;
    debug!("Successfully deleted container {}", c_id);
    Ok(())
}

/// Delete an object.
pub fn delete_object<C, O>(session: &Session, container: C, object: O) -> Result<()>
where
    C: AsRef<str>,
    O: AsRef<str>,
{
    let c_id = container.as_ref();
    let o_id = object.as_ref();
    debug!("Deleting object {} in container {}", o_id, c_id);
    let _ = session.delete(OBJECT_STORAGE, &[c_id, o_id], None)?;
    debug!("Successfully deleted object {} in container {}", o_id, c_id);
    Ok(())
}

/// Get container metadata.
pub fn get_container<C>(session: &Session, container: C) -> Result<Container>
where
    C: AsRef<str>,
{
    let c_id = container.as_ref();
    trace!("Requesting container {}", c_id);
    let resp =
        session.send_checked(session.request(OBJECT_STORAGE, Method::HEAD, &[c_id], None)?)?;
    let result = Container::from_headers(c_id, resp.headers())?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// Get object metadata.
pub fn get_object<C, O>(session: &Session, container: C, object: O) -> Result<Object>
where
    C: AsRef<str>,
    O: AsRef<str>,
{
    let c_id = container.as_ref();
    let o_id = object.as_ref();
    trace!("Requesting object {} from container {}", o_id, c_id);
    let resp = session.send_checked(session.request(
        OBJECT_STORAGE,
        Method::HEAD,
        &[c_id, o_id],
        None,
    )?)?;
    let result = Object::from_headers(o_id, resp.headers())?;
    trace!("Received {:?}", result);
    Ok(result)
}

/// Download the requested object.
pub fn download_object<C, O>(
    session: &Session,
    container: C,
    object: O,
) -> Result<SyncStream<impl Stream<Item = SyncStreamItem>>>
where
    C: AsRef<str>,
    O: AsRef<str>,
{
    let c_id = container.as_ref();
    let o_id = object.as_ref();
    trace!("Downloading object {} from container {}", o_id, c_id);
    Ok(session.download(session.get(OBJECT_STORAGE, &[c_id, o_id], None)?))
}

/// List containers for the current account.
pub fn list_containers(session: &Session, mut query: Query) -> Result<Vec<Container>> {
    query.push_str("format", "json");
    trace!("Listing containers with {:?}", query);
    let root: Vec<Container> = session.get_json_query(OBJECT_STORAGE, NO_PATH, query, None)?;
    trace!("Received containers: {:?}", root);
    Ok(root)
}

/// List objects in a given container.
pub fn list_objects<C>(session: &Session, container: C, mut query: Query) -> Result<Vec<Object>>
where
    C: AsRef<str>,
{
    query.push_str("format", "json");
    let id = container.as_ref();
    trace!("Listing objects in container {} with {:?}", id, query);
    let root: Vec<Object> = session.get_json_query(OBJECT_STORAGE, &[id], query, None)?;
    trace!("Received objects: {:?}", root);
    Ok(root)
}
