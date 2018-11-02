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
use std::sync::{Once, ONCE_INIT};

use waiter::Waiter;

use openstack::Refresh;


static INIT: Once = ONCE_INIT;

fn set_up() -> openstack::Cloud {
    INIT.call_once(|| { env_logger::init(); });

    openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment")
}

#[test]
fn test_port_create_update_delete() {
    let os = set_up();
    let network_id = env::var("RUST_OPENSTACK_NETWORK").expect("Missing RUST_OPENSTACK_NETWORK");

    let mut port = os.new_port(network_id.clone())
        .with_name("rust-openstack-integration")
        .create().expect("Could not create port");
    assert_eq!(port.name().as_ref().unwrap(), "rust-openstack-integration");
    assert!(port.device_id().is_none());
    assert!(port.device_owner().is_none());
    assert!(port.dns_name().is_none());
    assert!(port.dns_domain().is_none());
    assert!(!port.fixed_ips().is_empty());
    assert!(!port.is_dirty());
    assert_eq!(port.status(), openstack::network::NetworkStatus::Down);

    port.set_name("rust-openstack-integration-2");
    port.extra_dhcp_opts_mut().push(
        openstack::network::PortExtraDhcpOption::new("bootfile-name",
                                                     "pxelinux.0")
    );
    assert!(port.is_dirty());

    port.save().expect("Cannot update port");
    assert_eq!(port.name().as_ref().unwrap(), "rust-openstack-integration-2");
    assert_eq!(1, port.extra_dhcp_opts().len());
    assert!(!port.is_dirty());

    port.refresh().expect("Cannot refresh port");
    assert_eq!(port.name().as_ref().unwrap(), "rust-openstack-integration-2");
    assert!(!port.is_dirty());

    port.set_name("rust-openstack-integration-3");
    port.refresh().expect("Cannot refresh port");
    assert_eq!(port.name().as_ref().unwrap(), "rust-openstack-integration-2");
    assert!(!port.is_dirty());

    port.delete().expect("Cannot request port deletion")
        .wait().expect("Port was not deleted");

    os.get_port("rust-openstack-integration-2")
        .err().expect("Port is still present");
}

#[test]
fn test_network_create_delete_simple() {
    let os = set_up();

    let network = os.new_network().create().expect("Could not create network");
    assert!(network.admin_state_up());
    assert!(network.dns_domain().is_none());
    assert_eq!(network.external(), Some(false));
    assert!(!network.shared());
    assert!(network.name().is_none());

    network.delete().expect("Cannot request network deletion")
        .wait().expect("Network was not deleted");
}

#[test]
fn test_network_create_delete_with_fields() {
    let os = set_up();

    let network = os.new_network()
        .with_admin_state_up(false)
        .with_name("rust-openstack-integration-new")
        .with_mtu(1400)
        .with_description("New network for testing")
        .create().expect("Could not create network");
    assert!(!network.admin_state_up());
    assert!(network.dns_domain().is_none());
    assert_eq!(network.external(), Some(false));
    assert!(!network.shared());
    assert_eq!(network.name().as_ref().unwrap(), "rust-openstack-integration-new");

    network.delete().expect("Cannot request network deletion")
        .wait().expect("Network was not deleted");
}
