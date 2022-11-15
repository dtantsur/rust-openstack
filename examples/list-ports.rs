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
async fn display_port(port: &openstack::network::Port) {
    println!(
        "ID = {}, MAC = {}, UP = {}, Status = {}",
        port.id(),
        port.mac_address(),
        port.admin_state_up(),
        port.status()
    );
    println!(
        "* Owner = {:?}, Server? {}",
        port.device_owner(),
        port.attached_to_server()
    );
    for ip in port.fixed_ips() {
        let subnet = ip.subnet().await.expect("Cannot fetch subnet");
        println!("* IP = {}, Subnet = {}", ip.ip_address, subnet.cidr());
    }
    let net = port.network().await.expect("Cannot fetch network");
    println!("* Network: ID = {}, Name = {:?}", net.id(), net.name());
}

#[cfg(feature = "network")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");
    let sorting = openstack::network::PortSortKey::Name;

    let ports: Vec<_> = os
        .find_ports()
        .sort_by(openstack::Sort::Asc(sorting))
        .into_stream()
        .take(10)
        .try_collect()
        .await
        .expect("Cannot list ports");
    println!("First 10 ports:");
    for p in &ports {
        display_port(p).await;
    }

    let att_ports: Vec<_> = os
        .find_ports()
        .sort_by(openstack::Sort::Asc(
            openstack::network::PortSortKey::DeviceId,
        ))
        .with_status(openstack::network::NetworkStatus::Active)
        .all()
        .await
        .expect("Cannot list attached ports");
    println!("Only active ports attached to servers:");
    for p in &att_ports {
        if p.attached_to_server() {
            display_port(p).await;
        }
    }
}

#[cfg(not(feature = "network"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
