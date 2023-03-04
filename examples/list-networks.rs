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

use futures::stream::{StreamExt, TryStreamExt};

#[cfg(feature = "network")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");
    let sorting = openstack::network::NetworkSortKey::Name;

    let networks: Vec<openstack::network::Network> = os
        .find_networks()
        .sort_by(openstack::Sort::Asc(sorting))
        .into_stream()
        .take(10)
        .try_collect()
        .await
        .expect("Cannot list networks");
    println!("First 10 networks:");
    for s in &networks {
        println!(
            "ID = {}, Name = {:?}, UP = {}",
            s.id(),
            s.name(),
            s.admin_state_up()
        );
    }
}

#[cfg(not(feature = "network"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
