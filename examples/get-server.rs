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

use std::env;

#[cfg(feature = "compute")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");

    let id = env::args().nth(1).expect("Provide a server ID");
    let server = os.get_server(id).await.expect("Cannot get a server");

    println!(
        "ID = {}, Name = {}, Status = {:?}, Power = {:?}",
        server.id(),
        server.name(),
        server.status(),
        server.power_state()
    );
    println!("Links: image = {:?}", server.image_id());
    println!(
        "Flavor: {} CPU, disk {}G, memory {}M",
        server.flavor().vcpu_count,
        server.flavor().root_size,
        server.flavor().ram_size
    );
    println!("Floating IP: {:?}", server.floating_ip());

    if !server.metadata().is_empty() {
        println!("Metadata: {:?}", server.metadata());
    }
}

#[cfg(not(feature = "compute"))]
fn main() {
    panic!("This example cannot run with 'compute' feature disabled");
}
