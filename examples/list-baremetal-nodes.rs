// Copyright 2025 Dmitry Tantsur <divius.inside@gmail.com>
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

use futures::stream::{StreamExt, TryStreamExt};

#[cfg(feature = "baremetal")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");
    let sorting = openstack::baremetal::NodeSortKey::Name;

    let nodes: Vec<openstack::baremetal::Node> = os
        .find_baremetal_nodes()
        .sort_by(openstack::Sort::Asc(sorting))
        .detailed()
        .into_stream()
        .take(10)
        .try_collect()
        .await
        .expect("Cannot list servers");
    println!("First 10 nodes:");
    for s in &nodes {
        println!(
            "ID = {}, Name = {}",
            s.id(),
            s.name().clone().unwrap_or_default()
        );
    }

    let state = openstack::baremetal::NodeFilter::ProvisionState(
        openstack::baremetal::ProvisionState::Available,
    );
    let available = os
        .find_baremetal_nodes()
        .sort_by(openstack::Sort::Asc(sorting))
        .with(state)
        .all()
        .await
        .expect("Cannot list nodes");
    println!("All available nodes:");
    for s in &available {
        println!(
            "ID = {}, Name = {}",
            s.id(),
            s.name().clone().unwrap_or_default()
        );
    }
}

#[cfg(not(feature = "baremetal"))]
fn main() {
    panic!("This example cannot run with 'baremetal' feature disabled");
}
