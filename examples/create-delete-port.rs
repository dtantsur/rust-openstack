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

extern crate env_logger;
extern crate openstack;
extern crate waiter;

use std::env;
use waiter::Waiter;


#[cfg(feature = "network")]
fn display_port(port: &openstack::network::Port) {
    println!("ID = {}, Name = {:?}, MAC = {}, UP = {}, Status = {}",
             port.id(), port.name(), port.mac_address(),
             port.admin_state_up(), port.status());
    println!("* Owner = {:?}, Server? {}",
             port.device_owner(), port.attached_to_server());
    for ip in port.fixed_ips() {
        let subnet = ip.subnet().expect("Cannot fetch subnet");
        println!("* IP = {}, Subnet = {}", ip.ip_address, subnet.cidr());
    }
    let net = port.network().expect("Cannot fetch network");
    println!("* Network: ID = {}, Name = {}", net.id(), net.name());
}

#[cfg(feature = "network")]
fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment");

    let network = env::args().nth(1).expect("Provide a network");
    let subnet = env::args().nth(2).expect("Provide a subnet");

    let mut port = os.new_port(network).with_name("example-port")
        .with_fixed_ip(openstack::network::PortIpRequest::AnyIpFromSubnet(subnet.into()))
        .create().expect("Cannot create a port");

    display_port(&port);

    os.get_port("example-port").expect("Cannot find the port");

    println!("Updating the port");
    port.set_name("example-new-name");
    port.save().expect("Cannot update port");
    display_port(&port);

    port.delete().expect("Cannot request port deletion")
        .wait().expect("Port was not deleted");
}

#[cfg(not(feature = "network"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
