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
fn display_floating_ip(floating_ip: &openstack::network::FloatingIp) {
    println!("ID = {}, IP = {}, Fixed IP = {:?}, Status = {}",
             floating_ip.id(), floating_ip.floating_ip_address(),
             floating_ip.fixed_ip_address(), floating_ip.status());
    println!("* Network = {}, Name = {:?}",
             floating_ip.floating_network_id(),
             floating_ip.floating_network()
                .expect("Cannot fetch floating network").name());
    if floating_ip.is_associated() {
        println!("* Port = {}",
                 floating_ip.port().expect("Cannot fetch port").id());
    }
}

#[cfg(feature = "network")]
fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment");
    let sorting = openstack::network::FloatingIpSortKey::Status;

    let floating_ips: Vec<_> = os.find_floating_ips()
        .sort_by(openstack::Sort::Asc(sorting))
        .into_iter().take(10).collect()
        .expect("Cannot list floating IPs");
    println!("First 10 floating IPs:");
    for p in &floating_ips {
        display_floating_ip(p)
    }

    let att_floating_ips: Vec<_> = os.find_floating_ips()
        .sort_by(openstack::Sort::Asc(openstack::network::FloatingIpSortKey::FloatingIpAddress))
        .with_status(openstack::network::FloatingIpStatus::Active).all()
        .expect("Cannot list attached floating_ips");
    println!("Only active floating IPs:");
    for p in &att_floating_ips {
        display_floating_ip(p)
    }
}

#[cfg(not(feature = "network"))]
fn main() {
    panic!("This example cannot run with 'network' feature disabled");
}
