// Copyright 2024 Sandro-Alessio Gierens <sandro@gierens.de>
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

#[cfg(feature = "block-storage")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");
    let sorting = openstack::block_storage::VolumeSortKey::Name;

    let volumes: Vec<openstack::block_storage::Volume> = os
        .find_volumes()
        .sort_by(openstack::Sort::Asc(sorting))
        .into_stream()
        .take(10)
        .try_collect()
        .await
        .expect("Cannot list volumes");
    println!("First 10 volumes:");
    for s in &volumes {
        println!(
            "ID = {}, Name = {:?}, Status = {:?}",
            s.id(),
            s.name(),
            s.status(),
        );
    }
}

#[cfg(not(feature = "block-storage"))]
fn main() {
    panic!("This example cannot run with 'block-storage' feature disabled");
}
