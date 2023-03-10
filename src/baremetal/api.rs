// Copyright 2023 Dmitry Tantsur <dtantsur@protonmail.com>
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

use osauth::services::BAREMETAL;
use osauth::{ApiVersion, Error, ErrorKind, Query, Session};

use crate::Result;

use super::{constants::*, protocol::*, types::*};

async fn node_api_version(session: &Session) -> Result<ApiVersion> {
    session
        .pick_api_version(
            BAREMETAL,
            vec![
                API_VERSION_MINIMUM,
                API_VERSION_AUTOMATED_CLEAN,
                API_VERSION_PROTECTED,
                API_VERSION_CONDUCTORS,
                API_VERSION_OWNER,
                API_VERSION_DESCRIPTION,
                API_VERSION_ALLOCATIONS,
                API_VERSION_RETIRED,
                API_VERSION_LESSEE,
                API_VERSION_NETWORK_DATA,
                API_VERSION_BOOT_MODE,
                API_VERSION_SHARDS,
            ],
        )
        .await?
        .ok_or_else(|| {
            Error::new(
                ErrorKind::IncompatibleApiVersion,
                "BareMetal API version 1.46 (Rocky) or newer is required",
            )
        })
}

fn node_query_version(query: &Query<NodeFilter>) -> ApiVersion {
    let mut result = API_VERSION_MINIMUM;
    for item in &query.0 {
        let required_version = match item {
            NodeFilter::DescriptionContains(..) => API_VERSION_DESCRIPTION,
            NodeFilter::Lessee(..) => API_VERSION_LESSEE,
            NodeFilter::Owner(..) => API_VERSION_OWNER,
            NodeFilter::Project(..) => API_VERSION_LESSEE,
            NodeFilter::Retired(..) => API_VERSION_RETIRED,
            NodeFilter::Sharded(..) | NodeFilter::ShardIn(..) => API_VERSION_SHARDS,
            NodeFilter::SortKey(key) => match key {
                NodeSortKey::AutomatedClean => API_VERSION_AUTOMATED_CLEAN,
                NodeSortKey::Protected => API_VERSION_PROTECTED,
                NodeSortKey::Owner => API_VERSION_OWNER,
                NodeSortKey::Description => API_VERSION_DESCRIPTION,
                NodeSortKey::AllocationID => API_VERSION_ALLOCATIONS,
                NodeSortKey::Retired => API_VERSION_RETIRED,
                NodeSortKey::Lessee => API_VERSION_LESSEE,
                NodeSortKey::Shard => API_VERSION_SHARDS,
                _ => API_VERSION_MINIMUM,
            },
            NodeFilter::IncludeChildren(..) | NodeFilter::ParentNode(..) => API_VERSION_CHILD_NODES,
            _ => API_VERSION_MINIMUM,
        };
        result = std::cmp::max(result, required_version);
    }
    result
}

/// Get a node.
pub async fn get_node<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<Node> {
    let api_version = node_api_version(session).await?;
    let root: Node = session
        .get(BAREMETAL, &["nodes", id_or_name.as_ref()])
        .api_version(api_version)
        .fetch()
        .await?;
    trace!("Received {:?}", root);
    Ok(root)
}

/// List nodes.
pub async fn list_nodes(session: &Session, query: &Query<NodeFilter>) -> Result<Vec<NodeSummary>> {
    trace!("Listing baremetal nodes with {:?}", query);
    let api_version = node_query_version(query);
    let root: NodesRoot = session
        .get(BAREMETAL, &["nodes"])
        .api_version(api_version)
        .query(query)
        .fetch()
        .await?;
    trace!("Received baremetal nodes: {:?}", root.nodes);
    Ok(root.nodes)
}

/// List nodes detailed.
pub async fn list_nodes_detailed(
    session: &Session,
    query: &Query<NodeFilter>,
) -> Result<Vec<Node>> {
    trace!("Listing baremetal nodes with {:?}", query);
    let api_version = node_query_version(query);
    let root: NodesDetailRoot = session
        .get(BAREMETAL, &["nodes", "detail"])
        .api_version(api_version)
        .query(query)
        .fetch()
        .await?;
    trace!("Received baremetal nodes: {:?}", root.nodes);
    Ok(root.nodes)
}
