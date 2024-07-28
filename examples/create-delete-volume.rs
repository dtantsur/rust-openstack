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

use std::env;

#[cfg(feature = "block-storage")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");

    let name = env::args().nth(1).expect("Provide a volume name");
    let size = env::args()
        .nth(2)
        .expect("Provide a volume size in GiB")
        .parse::<u64>()
        .expect("Size must be an integer");

    let volume = os
        .new_volume(size)
        .with_name(name)
        .create()
        .await
        .expect("Cannot create a volume");

    println!(
        "ID = {}, Name = {}, Status = {:?}",
        volume.id(),
        volume.name(),
        volume.status(),
    );

    volume.delete().await.expect("Failed to delete volume.");
}

#[cfg(not(feature = "block-storage"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
