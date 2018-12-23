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
extern crate fallible_iterator;
extern crate openstack;

use fallible_iterator::FallibleIterator;

#[cfg(feature = "network")]
fn display_subnet(subnet: &openstack::network::Subnet) {
    println!(
        "ID = {}, CIDR = {}, Gateway = {:?}, DHCP? {}",
        subnet.id(),
        subnet.cidr(),
        subnet.gateway_ip(),
        subnet.dhcp_enabled()
    );
    let net = subnet.network().expect("Cannot fetch network");
    println!("* Network: ID = {}, Name = {:?}", net.id(), net.name());
}

#[cfg(feature = "network")]
fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment");
    let sorting = openstack::network::SubnetSortKey::Name;

    let subnets: Vec<_> = os
        .find_subnets()
        .sort_by(openstack::Sort::Asc(sorting))
        .into_iter()
        .take(10)
        .collect()
        .expect("Cannot list subnets");
    println!("First 10 subnets:");
    for p in &subnets {
        display_subnet(p)
    }
}

#[cfg(not(feature = "network"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
