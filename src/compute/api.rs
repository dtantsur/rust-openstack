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

//! Foundation bits exposing the Compute API.

use std::collections::HashMap;
use std::fmt::Debug;

use osauth::services::COMPUTE;
use osproto::common::{IdAndName, Ref};
use serde::Serialize;
use serde_json;

use super::super::common::ApiVersion;
use super::super::session::Session;
use super::super::utils::{self, ResultExt};
use super::super::Result;
use super::protocol::*;

const API_VERSION_KEYPAIR_TYPE: ApiVersion = ApiVersion(2, 2);
const API_VERSION_SERVER_DESCRIPTION: ApiVersion = ApiVersion(2, 19);
const API_VERSION_KEYPAIR_PAGINATION: ApiVersion = ApiVersion(2, 35);
const API_VERSION_FLAVOR_DESCRIPTION: ApiVersion = ApiVersion(2, 55);
const API_VERSION_FLAVOR_EXTRA_SPECS: ApiVersion = ApiVersion(2, 61);

fn flavor_api_version(session: &Session) -> Result<Option<ApiVersion>> {
    session.pick_api_version(
        COMPUTE,
        vec![
            API_VERSION_FLAVOR_DESCRIPTION,
            API_VERSION_FLAVOR_EXTRA_SPECS,
        ],
    )
}

/// Create a key pair.
pub fn create_keypair(session: &Session, request: KeyPairCreate) -> Result<KeyPair> {
    let version = if request.key_type.is_some() {
        Some(API_VERSION_KEYPAIR_TYPE)
    } else {
        None
    };

    debug!("Creating a key pair with {:?}", request);
    let body = KeyPairCreateRoot { keypair: request };
    let root: KeyPairRoot = session.post_json(COMPUTE, &["os-keypairs"], body, version)?;
    debug!("Created key pair {:?}", root.keypair);
    Ok(root.keypair)
}

/// Create a server.
pub fn create_server(session: &Session, request: ServerCreate) -> Result<Ref> {
    debug!("Creating a server with {:?}", request);
    let body = ServerCreateRoot { server: request };
    let root: CreatedServerRoot = session.post_json(COMPUTE, &["servers"], body, None)?;
    trace!("Requested creation of server {:?}", root.server);
    Ok(root.server)
}

/// Delete a key pair.
pub fn delete_keypair<S: AsRef<str>>(session: &Session, name: S) -> Result<()> {
    debug!("Deleting key pair {}", name.as_ref());
    let _ = session.delete(COMPUTE, &["os-keypairs", name.as_ref()], None)?;
    debug!("Key pair {} was deleted", name.as_ref());
    Ok(())
}

/// Delete a server.
pub fn delete_server<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    trace!("Deleting server {}", id.as_ref());
    let _ = session.delete(COMPUTE, &["servers", id.as_ref()], None)?;
    debug!("Successfully requested deletion of server {}", id.as_ref());
    Ok(())
}

/// Get a flavor by its ID.
pub fn get_extra_specs_by_flavor_id<S: AsRef<str>>(
    session: &Session,
    id: S,
) -> Result<HashMap<String, String>> {
    trace!("Get compute extra specs by ID {}", id.as_ref());
    let root: ExtraSpecsRoot =
        session.get_json(COMPUTE, &["flavors", id.as_ref(), "os-extra_specs"], None)?;
    trace!("Received {:?}", root.extra_specs);
    Ok(root.extra_specs)
}

/// Get a flavor.
pub fn get_flavor<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Flavor> {
    let s = id_or_name.as_ref();
    get_flavor_by_id(session, s).if_not_found_then(|| get_flavor_by_name(session, s))
}

/// Get a flavor by its ID.
pub fn get_flavor_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<Flavor> {
    trace!("Get compute flavor by ID {}", id.as_ref());
    let version = flavor_api_version(session)?;
    let root: FlavorRoot = session.get_json(COMPUTE, &["flavors", id.as_ref()], version)?;
    trace!("Received {:?}", root.flavor);
    Ok(root.flavor)
}

/// Get a flavor by its name.
pub fn get_flavor_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Flavor> {
    trace!("Get compute flavor by name {}", name.as_ref());
    let root: FlavorsRoot = session.get_json(COMPUTE, &["flavors"], None)?;
    utils::one(
        root.flavors
            .into_iter()
            .filter(|item| item.name == name.as_ref()),
        "Flavor with given name or ID not found",
        "Too many flavors found with given name",
    )
    .and_then(|item| get_flavor_by_id(session, item.id))
}

/// Get a key pair by its name.
pub fn get_keypair<S: AsRef<str>>(session: &Session, name: S) -> Result<KeyPair> {
    trace!("Get compute key pair by name {}", name.as_ref());
    let ver = session.pick_api_version(COMPUTE, Some(API_VERSION_KEYPAIR_TYPE))?;
    let root: KeyPairRoot = session.get_json(COMPUTE, &["os-keypairs", name.as_ref()], ver)?;
    trace!("Received {:?}", root.keypair);
    Ok(root.keypair)
}

/// Get a server.
pub fn get_server<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Server> {
    let s = id_or_name.as_ref();
    get_server_by_id(session, s).if_not_found_then(|| get_server_by_name(session, s))
}

/// Get a server by its ID.
pub fn get_server_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<Server> {
    trace!("Get compute server with ID {}", id.as_ref());
    let version = session.pick_api_version(COMPUTE, Some(API_VERSION_SERVER_DESCRIPTION))?;
    let root: ServerRoot = session.get_json(COMPUTE, &["servers", id.as_ref()], version)?;
    trace!("Received {:?}", root.server);
    Ok(root.server)
}

/// Get a server by its name.
pub fn get_server_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<Server> {
    trace!("Get compute server with name {}", name.as_ref());
    let root: ServersRoot =
        session.get_json_query(COMPUTE, &["servers"], &[("name", name.as_ref())], None)?;
    utils::one(
        root.servers
            .into_iter()
            .filter(|item| item.name == name.as_ref()),
        "Server with given name or ID not found",
        "Too many servers found with given name",
    )
    .and_then(|item| get_server_by_id(session, item.id))
}

/// List flavors.
pub fn list_flavors<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<IdAndName>> {
    trace!("Listing compute flavors with {:?}", query);
    let root: FlavorsRoot = session.get_json_query(COMPUTE, &["flavors"], query, None)?;
    trace!("Received flavors: {:?}", root.flavors);
    Ok(root.flavors)
}

/// List flavors with details.
pub fn list_flavors_detail<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<Flavor>> {
    trace!("Listing compute flavors with {:?}", query);
    let version = session.pick_api_version(COMPUTE, Some(API_VERSION_FLAVOR_EXTRA_SPECS))?;
    let root: FlavorsDetailRoot =
        session.get_json_query(COMPUTE, &["flavors", "detail"], query, version)?;
    trace!("Received flavors: {:?}", root.flavors);
    Ok(root.flavors)
}

/// List key pairs.
pub fn list_keypairs<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<KeyPair>> {
    trace!("Listing compute key pairs with {:?}", query);
    let ver = session.pick_api_version(
        COMPUTE,
        vec![API_VERSION_KEYPAIR_TYPE, API_VERSION_KEYPAIR_PAGINATION],
    )?;
    let root: KeyPairsRoot = session.get_json_query(COMPUTE, &["os-keypairs"], query, ver)?;
    let result = root
        .keypairs
        .into_iter()
        .map(|item| item.keypair)
        .collect::<Vec<_>>();
    trace!("Received key pairs: {:?}", result);
    Ok(result)
}

/// List servers.
pub fn list_servers<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<IdAndName>> {
    trace!("Listing compute servers with {:?}", query);
    let root: ServersRoot = session.get_json_query(COMPUTE, &["servers"], query, None)?;
    trace!("Received servers: {:?}", root.servers);
    Ok(root.servers)
}

/// List servers with details.
pub fn list_servers_detail<Q: Serialize + Sync + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<Server>> {
    trace!("Listing compute servers with {:?}", query);
    let version = session.pick_api_version(COMPUTE, Some(API_VERSION_SERVER_DESCRIPTION))?;
    let root: ServersDetailRoot =
        session.get_json_query(COMPUTE, &["servers", "detail"], query, version)?;
    trace!("Received servers: {:?}", root.servers);
    Ok(root.servers)
}

/// Run an action while providing some arguments.
pub fn server_action_with_args<S1, S2, Q>(
    session: &Session,
    id: S1,
    action: S2,
    args: Q,
) -> Result<()>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
    Q: Serialize + Send + Debug,
{
    trace!(
        "Running {} on server {} with args {:?}",
        action.as_ref(),
        id.as_ref(),
        args
    );
    let mut body = HashMap::new();
    let _ = body.insert(action.as_ref(), args);
    let _ = session.post(COMPUTE, &["servers", id.as_ref(), "action"], body, None)?;
    debug!(
        "Successfully ran {} on server {}",
        action.as_ref(),
        id.as_ref()
    );
    Ok(())
}

/// Run an action on the server.
pub fn server_simple_action<S1, S2>(session: &Session, id: S1, action: S2) -> Result<()>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    server_action_with_args(session, id, action, serde_json::Value::Null)
}

/// Whether key pair pagination is supported.
#[inline]
pub fn supports_keypair_pagination(session: &Session) -> Result<bool> {
    session.supports_api_version(COMPUTE, API_VERSION_KEYPAIR_PAGINATION)
}
