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
extern crate ipnet;
extern crate openstack;
extern crate waiter;

use std::env;
use std::net;
use std::sync::Once;

use fallible_iterator::FallibleIterator;
use waiter::Waiter;

use openstack::Refresh;

static INIT: Once = Once::new();

fn set_up() -> openstack::Cloud {
    INIT.call_once(|| {
        env_logger::init();
    });

    openstack::Cloud::from_env()
        .expect("Failed to create an identity provider from the environment")
}

#[test]
fn test_port_create_update_delete() {
    let os = set_up();
    let network_id = env::var("RUST_OPENSTACK_NETWORK").expect("Missing RUST_OPENSTACK_NETWORK");

    let mut port = os
        .new_port(network_id.clone())
        .with_name("rust-openstack-integration")
        .create()
        .expect("Could not create port");
    assert_eq!(port.name().as_ref().unwrap(), "rust-openstack-integration");
    assert!(port.device_id().is_none());
    assert!(port.device_owner().is_none());
    assert!(port.dns_name().is_none());
    assert!(port.dns_domain().is_none());
    assert!(!port.fixed_ips().is_empty());
    assert!(!port.is_dirty());
    assert_eq!(port.status(), openstack::network::NetworkStatus::Down);

    port.set_name("rust-openstack-integration-2");
    port.extra_dhcp_opts_mut()
        .push(openstack::network::PortExtraDhcpOption::new(
            "bootfile-name",
            "pxelinux.0",
        ));
    assert!(port.is_dirty());

    port.save().expect("Cannot update port");
    assert_eq!(
        port.name().as_ref().unwrap(),
        "rust-openstack-integration-2"
    );
    assert_eq!(1, port.extra_dhcp_opts().len());
    assert!(!port.is_dirty());

    port.refresh().expect("Cannot refresh port");
    assert_eq!(
        port.name().as_ref().unwrap(),
        "rust-openstack-integration-2"
    );
    assert!(!port.is_dirty());

    port.set_name("rust-openstack-integration-3");
    port.refresh().expect("Cannot refresh port");
    assert_eq!(
        port.name().as_ref().unwrap(),
        "rust-openstack-integration-2"
    );
    assert!(!port.is_dirty());

    let mut port_found = os
        .find_ports()
        .with_network(network_id)
        .with_name("rust-openstack-integration-2")
        .one()
        .expect("Cannot find port by network");
    assert_eq!(
        port_found.name().as_ref().unwrap(),
        "rust-openstack-integration-2"
    );
    assert!(!port_found.is_dirty());

    port.delete()
        .expect("Cannot request port deletion")
        .wait()
        .expect("Port was not deleted");

    os.get_port("rust-openstack-integration-2")
        .err()
        .expect("Port is still present");

    port_found
        .refresh()
        .err()
        .expect("Refresh succeeds on deleted port");
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
    assert_eq!(network.status(), openstack::network::NetworkStatus::Active);

    let cidr = ipnet::Ipv4Net::new(net::Ipv4Addr::new(192, 168, 1, 0), 24)
        .unwrap()
        .into();
    let mut subnet = os
        .new_subnet(network.clone(), cidr)
        .create()
        .expect("Could not create subnet");
    assert_eq!(subnet.cidr(), cidr);
    assert!(subnet.dhcp_enabled());
    assert!(subnet.dns_nameservers().is_empty());
    assert_eq!(subnet.ip_version(), openstack::network::IpVersion::V4);
    assert!(subnet.name().is_none());

    subnet.refresh().expect("Cannot refresh subnet");

    subnet
        .delete()
        .expect("Cannot request subnet deletion")
        .wait()
        .expect("Subnet was not deleted");

    network
        .delete()
        .expect("Cannot request network deletion")
        .wait()
        .expect("Network was not deleted");
}

#[test]
fn test_subnet_update() {
    let os = set_up();

    let network = os.new_network().create().expect("Could not create network");
    let cidr = ipnet::Ipv4Net::new(net::Ipv4Addr::new(192, 168, 1, 0), 24)
        .unwrap()
        .into();
    let mut subnet = os
        .new_subnet(network.clone(), cidr)
        .create()
        .expect("Could not create subnet");
    assert!(!subnet.is_dirty());

    subnet.dns_nameservers_mut().push("1.2.3.4".to_string());
    assert!(subnet.is_dirty());
    subnet.set_name("unused name");
    subnet.set_description("unused description");

    subnet.refresh().expect("Cannot refresh subnet");

    assert!(!subnet.is_dirty());
    assert!(subnet.dns_nameservers().is_empty());
    assert!(subnet.name().is_none());

    subnet.dns_nameservers_mut().push("8.8.8.8".to_string());
    subnet.set_dhcp_enabled(false);
    subnet.set_name("rust-openstack-integration-new");
    subnet.set_description("some description");

    assert_eq!("8.8.8.8", subnet.dns_nameservers()[0]);
    assert!(!subnet.dhcp_enabled());
    assert_eq!(
        subnet.name().as_ref().unwrap(),
        "rust-openstack-integration-new"
    );

    subnet.save().expect("Could not save subnet");

    assert!(!subnet.is_dirty());
    assert_eq!("8.8.8.8", subnet.dns_nameservers()[0]);
    assert!(!subnet.dhcp_enabled());
    assert_eq!(
        subnet.name().as_ref().unwrap(),
        "rust-openstack-integration-new"
    );

    subnet.refresh().expect("Cannot refresh subnet");

    assert_eq!("8.8.8.8", subnet.dns_nameservers()[0]);
    assert!(!subnet.dhcp_enabled());
    assert_eq!(
        subnet.name().as_ref().unwrap(),
        "rust-openstack-integration-new"
    );

    subnet
        .delete()
        .expect("Cannot request subnet deletion")
        .wait()
        .expect("Subnet was not deleted");

    network
        .delete()
        .expect("Cannot request network deletion")
        .wait()
        .expect("Network was not deleted");
}

#[test]
fn test_network_update() {
    let os = set_up();

    let mut network = os
        .new_network()
        .with_admin_state_up(false)
        .with_name("rust-openstack-integration-new")
        .with_mtu(1400)
        .with_description("New network for testing")
        .create()
        .expect("Could not create network");
    assert!(!network.is_dirty());

    network.set_admin_state_up(true);
    network.set_name("rust-openstack-integration-new2");
    network.set_mtu(1450);

    assert!(network.is_dirty());
    assert!(network.admin_state_up());
    assert_eq!(network.mtu(), Some(1450));
    assert_eq!(
        network.name().as_ref().unwrap(),
        "rust-openstack-integration-new2"
    );

    network.save().expect("Could not save network");

    assert!(!network.is_dirty());
    assert!(network.admin_state_up());
    assert_eq!(network.mtu(), Some(1450));
    assert_eq!(
        network.name().as_ref().unwrap(),
        "rust-openstack-integration-new2"
    );

    network.set_name("rust-openstack-integration-new3");
    network.set_mtu(42);
    assert!(network.is_dirty());

    network.refresh().expect("Could not refresh network");

    assert!(!network.is_dirty());
    assert!(network.admin_state_up());
    assert_eq!(network.mtu(), Some(1450));
    assert_eq!(
        network.name().as_ref().unwrap(),
        "rust-openstack-integration-new2"
    );

    network
        .delete()
        .expect("Cannot request network deletion")
        .wait()
        .expect("Network was not deleted");
}

#[test]
fn test_network_create_delete_with_fields() {
    let os = set_up();

    let network = os
        .new_network()
        .with_admin_state_up(false)
        .with_name("rust-openstack-integration-new")
        .with_mtu(1400)
        .with_description("New network for testing")
        .create()
        .expect("Could not create network");
    assert!(!network.admin_state_up());
    assert!(network.dns_domain().is_none());
    assert_eq!(network.external(), Some(false));
    assert!(!network.shared());
    assert_eq!(
        network.name().as_ref().unwrap(),
        "rust-openstack-integration-new"
    );

    let cidr = ipnet::Ipv4Net::new(net::Ipv4Addr::new(192, 168, 1, 0), 24)
        .unwrap()
        .into();
    let subnet = os
        .new_subnet("rust-openstack-integration-new", cidr)
        .with_name("rust-openstack-integration-new")
        .with_dhcp_enabled(false)
        .with_dns_nameserver("8.8.8.8")
        .create()
        .expect("Could not create subnet");
    assert_eq!(subnet.cidr(), cidr);
    assert!(!subnet.dhcp_enabled());
    assert_eq!(subnet.dns_nameservers(), &vec!["8.8.8.8".to_string()]);
    assert_eq!(subnet.ip_version(), openstack::network::IpVersion::V4);
    assert_eq!(
        subnet.name().as_ref().unwrap(),
        "rust-openstack-integration-new"
    );

    let subnets = os
        .find_subnets()
        .with_network("rust-openstack-integration-new")
        .all()
        .expect("Cannot find subnets by network name");
    assert_eq!(subnets.len(), 1);
    assert_eq!(subnets[0].id(), subnet.id());
    assert_eq!(subnets[0].name(), subnet.name());

    subnet
        .delete()
        .expect("Cannot request subnet deletion")
        .wait()
        .expect("Subnet was not deleted");

    network
        .delete()
        .expect("Cannot request network deletion")
        .wait()
        .expect("Network was not deleted");
}

#[test]
fn test_floating_ip_create_delete() {
    let os = set_up();
    let floating_network_id = env::var("RUST_OPENSTACK_FLOATING_NETWORK")
        .expect("Missing RUST_OPENSTACK_FLOATING_NETWORK");

    let mut floating_ip = os
        .new_floating_ip(floating_network_id)
        .create()
        .expect("Cannot create a floating IP");
    assert!(!floating_ip.is_associated());
    floating_ip.port().err().unwrap();

    let net = floating_ip
        .floating_network()
        .expect("Cannot find floating network");
    let floating_ip_found = os
        .find_floating_ips()
        .with_floating_network(net.name().clone().expect("Floating network has no name"))
        .into_iter()
        .find(|ip| Ok(ip.id() == floating_ip.id()))
        .expect("Cannot list floating IPs")
        .expect("Floating IP was not found");
    assert_eq!(floating_ip_found.id(), floating_ip.id());

    floating_ip.refresh().expect("Cannot refresh a floating IP");

    floating_ip
        .delete()
        .expect("Cannot request floating IP deletion")
        .wait()
        .expect("Floating IP was not deleted");
}
#[test]
fn test_router_create_delete_simple() {
    let os = set_up();

    let router = os.new_router().create().expect("Could not create router.");
    assert!(router.admin_state_up());
    assert!(router.availability_zone_hints().is_empty());
    assert!(router.availability_zones().is_empty());
    assert!(router.created_at().is_some());
    assert!(router.conntrack_helpers().is_empty());
    assert!(router.description().is_none());
    assert!(router.distributed().is_none());
    assert!(router.external_gateway().is_none());
    assert!(router.flavor_id().is_none());
    assert!(router.ha().is_none());
    assert!(router.name().is_none());
    assert!(router.revision_number().is_some());
    assert_eq!(*router.routes(), Some(vec![]));
    assert!(router.service_type_id().is_none());
    assert_eq!(router.status(), openstack::network::RouterStatus::Active);

    let network = os.new_network().create().expect("Could not create network");
    let cidr = ipnet::Ipv4Net::new(net::Ipv4Addr::new(192, 168, 1, 0), 24)
        .unwrap()
        .into();
    let mut subnet = os
        .new_subnet(network.clone(), cidr)
        .create()
        .expect("Could not create subnet");

    subnet.refresh().expect("Cannot refresh subnet");

    let _ = router.delete();

    subnet
        .delete()
        .expect("Cannot request subnet deletion")
        .wait()
        .expect("Subnet was not deleted");

    network
        .delete()
        .expect("Cannot request network deletion")
        .wait()
        .expect("Network was not deleted");
}

#[test]
fn test_router_create_update_delete_with_fields() {
    let os = set_up();

    let mut router = os
        .new_router()
        .with_admin_state_up(false)
        .with_availability_zone_hints(vec![String::from("nova")])
        .with_description("rust openstack integration")
        .with_name("rust-openstack-integration")
        .create()
        .expect("Could not create router.");

    assert!(!router.admin_state_up());
    assert!(router.availability_zone_hints().is_empty());
    assert!(router.availability_zones().is_empty());
    assert!(router.created_at().is_some());
    assert!(router.conntrack_helpers().is_empty());
    assert_eq!(
        router.description(),
        &Some(String::from("rust openstack integration"))
    );
    assert!(router.distributed().is_none());
    assert!(router.external_gateway().is_none());
    assert!(router.flavor_id().is_none());
    assert!(router.ha().is_none());
    assert_eq!(
        router.name().as_ref().unwrap(),
        "rust-openstack-integration"
    );
    assert!(router.project_id().is_some());
    assert!(router.revision_number().is_some());
    assert_eq!(*router.routes(), Some(vec![]));
    assert!(router.service_type_id().is_none());
    assert_eq!(router.status(), openstack::network::RouterStatus::Active);

    let network = os
        .new_network()
        .with_admin_state_up(false)
        .with_name("rust-openstack-integration-new")
        .with_description("New network for testing")
        .create()
        .expect("Could not create network");

    let cidr = ipnet::Ipv4Net::new(net::Ipv4Addr::new(192, 168, 1, 0), 24)
        .unwrap()
        .into();
    let subnet = os
        .new_subnet("rust-openstack-integration-new", cidr)
        .with_name("rust-openstack-integration-new")
        .with_dhcp_enabled(false)
        .with_dns_nameserver("8.8.8.8")
        .create()
        .expect("Could not create subnet");

    let ports = os.find_ports().with_device_id(router.id()).all();
    assert_eq!(ports.unwrap().len(), 0);
    let _ = router.add_router_interface(Some(subnet.id()), None);
    let ports = os.find_ports().with_device_id(router.id()).all();
    assert_eq!(ports.unwrap().len(), 1);
    let _ = router.remove_router_interface(Some(subnet.id()), None);
    let ports = os.find_ports().with_device_id(router.id()).all();
    assert_eq!(ports.unwrap().len(), 0);

    let port = os.new_port(network.id().as_ref()).create().unwrap();
    let _ = router.add_router_interface(None, Some(port.id()));
    let ports = os.find_ports().with_device_id(router.id()).all();
    assert_eq!(ports.unwrap().len(), 1);
    let _ = router.remove_router_interface(None, Some(port.id()));
    let ports = os.find_ports().with_device_id(router.id()).all();
    assert_eq!(ports.unwrap().len(), 0);

    let routers = os
        .find_routers()
        .with_name("rust-openstack-integration")
        .all()
        .expect("Cannot find routers by router name.");
    assert_eq!(routers.len(), 1);
    assert_eq!(routers[0].id(), router.id());
    assert_eq!(routers[0].name(), router.name());

    subnet
        .delete()
        .expect("Cannot request subnet deletion")
        .wait()
        .expect("Subnet was not deleted");

    network
        .delete()
        .expect("Cannot request network deletion")
        .wait()
        .expect("Network was not deleted");

    router
        .delete()
        .expect("Cannot request router deletetion.")
        .wait()
        .expect("Router was not deleted.");
}

#[test]
fn test_router_update() {
    let os = set_up();

    let mut router = os
        .new_router()
        .with_admin_state_up(false)
        .with_name("rust-openstack-integration-new")
        .with_description("New router for testing")
        .create()
        .expect("Could not create router");
    assert!(!router.is_dirty());

    router.set_admin_state_up(true);
    router.set_name("rust-openstack-integration-new2");
    router.set_description("Updated router for testing.");

    assert!(router.is_dirty());
    assert!(router.admin_state_up());
    assert_eq!(
        router.name().as_ref().unwrap(),
        "rust-openstack-integration-new2"
    );
    assert_eq!(
        router.description().as_ref().unwrap(),
        "Updated router for testing."
    );

    router.save().expect("Could not save router");

    assert!(!router.is_dirty());
    assert!(router.admin_state_up());
    assert_eq!(
        router.name().as_ref().unwrap(),
        "rust-openstack-integration-new2"
    );
    assert_eq!(
        router.description().as_ref().unwrap(),
        "Updated router for testing."
    );

    router.set_name("rust-openstack-integration-new3");
    router.set_description("This will be reverted by a refresh.");
    assert!(router.is_dirty());

    router.refresh().expect("Could not refresh router.");

    assert!(!router.is_dirty());
    assert!(router.admin_state_up());
    assert_eq!(
        router.name().as_ref().unwrap(),
        "rust-openstack-integration-new2"
    );
    assert_eq!(
        router.description().as_ref().unwrap(),
        "Updated router for testing."
    );

    router
        .delete()
        .expect("Cannot request router deletion.")
        .wait()
        .expect("Router was not deleted.");
}
