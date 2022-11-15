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

use std::env;
use waiter::Waiter;

#[cfg(feature = "network")]
async fn display_port(port: &openstack::network::Port) {
    println!(
        "ID = {}, Name = {:?}, MAC = {}, UP = {}, Status = {}",
        port.id(),
        port.name(),
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

    let network = env::args().nth(1).expect("Provide a network");
    let subnet = env::args().nth(2).expect("Provide a subnet");

    let mut port = os
        .new_port(network)
        .with_name("example-port")
        .with_fixed_ip(openstack::network::PortIpRequest::AnyIpFromSubnet(
            subnet.into(),
        ))
        .create()
        .await
        .expect("Cannot create a port");

    display_port(&port).await;

    os.get_port("example-port")
        .await
        .expect("Cannot find the port");

    println!("Updating the port");
    port.set_name("example-new-name");
    port.save().await.expect("Cannot update port");
    display_port(&port).await;

    port.delete()
        .await
        .expect("Cannot request port deletion")
        .wait()
        .await
        .expect("Port was not deleted");
}

#[cfg(not(feature = "network"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
