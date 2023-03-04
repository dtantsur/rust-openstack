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

#[cfg(feature = "object-storage")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");

    let containers: Vec<openstack::object_storage::Container> =
        os.list_containers().await.expect("Cannot list containers");
    println!("Containers:");
    for container in &containers {
        println!(
            "Name = {}, Bytes = {}, Number of objects = {}",
            container.name(),
            container.bytes(),
            container.object_count()
        );
    }
}

#[cfg(not(feature = "object-storage"))]
fn main() {
    panic!("This example cannot run with 'object-storage' feature disabled");
}
