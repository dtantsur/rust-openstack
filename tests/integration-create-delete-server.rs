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
use std::fs::File;
use std::io::Read;
use std::sync::Once;

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

fn validate_port(port: &openstack::network::Port, server: &openstack::compute::Server) {
    assert_eq!(port.device_id().as_ref().unwrap(), server.id());
    assert!(port
        .device_owner()
        .as_ref()
        .unwrap()
        .starts_with("compute:"));
    assert!(port.attached_to_server());
    assert!(port.fixed_ips().len() > 0);
}

fn power_on_off_server(server: &mut openstack::compute::Server) {
    server
        .stop()
        .expect("Failed to request power off")
        .wait()
        .expect("Failed to power off");
    assert_eq!(
        server.power_state(),
        openstack::compute::ServerPowerState::Shutdown
    );

    server
        .start()
        .expect("Failed to request power on")
        .wait()
        .expect("Failed to power on");
    assert_eq!(
        server.power_state(),
        openstack::compute::ServerPowerState::Running
    );
}

fn validate_server(os: &openstack::Cloud, server: &mut openstack::compute::Server) {
    assert_eq!(server.name(), "rust-openstack-integration");
    assert_eq!(server.status(), openstack::compute::ServerStatus::Active);
    assert_eq!(
        server.power_state(),
        openstack::compute::ServerPowerState::Running
    );
    assert_eq!(
        server.metadata().get("meta"),
        Some(&"a3f955c049f7416faa7".to_string())
    );

    power_on_off_server(server);

    let port = os
        .find_ports()
        .with_device_id(server.id().clone())
        .with_admin_state_up(true)
        .one()
        .expect("Cannot find the port attached to the server");
    validate_port(&port, &server);

    let image = server.image().expect("Cannot fetch Server image");
    assert_eq!(image.id(), server.image_id().unwrap());

    let flavor = server.flavor();
    assert!(flavor.vcpu_count > 0);
    assert!(flavor.ram_size > 0);
    assert!(flavor.root_size > 0);
}

#[test]
fn test_basic_server_ops() {
    let os = set_up();
    let image_id = env::var("RUST_OPENSTACK_IMAGE").expect("Missing RUST_OPENSTACK_IMAGE");
    let flavor_id = env::var("RUST_OPENSTACK_FLAVOR").expect("Missing RUST_OPENSTACK_FLAVOR");
    let network_id = env::var("RUST_OPENSTACK_NETWORK").expect("Missing RUST_OPENSTACK_NETWORK");
    let floating_network_id = env::var("RUST_OPENSTACK_FLOATING_NETWORK")
        .expect("Missing RUST_OPENSTACK_FLOATING_NETWORK");

    let (keypair, private_key) = os
        .new_keypair("rust-openstack-integration")
        .with_key_type(openstack::compute::KeyPairType::SSH)
        .generate()
        .expect("Cannot create a key pair");
    assert!(!private_key.is_empty());

    let mut server = os
        .new_server("rust-openstack-integration", flavor_id)
        .with_image(image_id)
        .with_network(network_id.clone())
        .with_keypair(keypair)
        .with_metadata("meta", "a3f955c049f7416faa7")
        .create()
        .expect("Failed to request server creation")
        .wait()
        .expect("Server was not created");

    validate_server(&os, &mut server);

    let ports = os
        .find_ports()
        .with_network(network_id)
        .with_status(openstack::network::NetworkStatus::Active)
        .all()
        .expect("Cannot find active ports for network");
    assert!(ports.len() > 0);

    let server_port = os
        .find_ports()
        .with_device_id(server.id().clone())
        .one()
        .expect("Cannot find the port attached to the server");

    let mut floating_ip = os
        .new_floating_ip(floating_network_id)
        .create()
        .expect("Cannot create a floating IP");

    floating_ip
        .associate(server_port, None)
        .expect("Cannot associate floating IP");

    floating_ip
        .delete()
        .expect("Failed to request floating IP deletion")
        .wait()
        .expect("Failed to delete floating IP");

    server
        .delete()
        .expect("Failed to request deletion")
        .wait()
        .expect("Failed to delete server");

    os.get_keypair("rust-openstack-integration")
        .expect("Cannot get key pair")
        .delete()
        .expect("Cannot delete key pair");
}

#[test]
fn test_server_ops_with_port() {
    let os = set_up();
    let image_id = env::var("RUST_OPENSTACK_IMAGE").expect("Missing RUST_OPENSTACK_IMAGE");
    let flavor_id = env::var("RUST_OPENSTACK_FLAVOR").expect("Missing RUST_OPENSTACK_FLAVOR");
    let network_id = env::var("RUST_OPENSTACK_NETWORK").expect("Missing RUST_OPENSTACK_NETWORK");
    let keypair_file_name =
        env::var("RUST_OPENSTACK_KEYPAIR").expect("Missing RUST_OPENSTACK_KEYPAIR");
    let mut keypair_pkey = String::new();
    let _ = File::open(keypair_file_name)
        .expect("Cannot open RUST_OPENSTACK_KEYPAIR")
        .read_to_string(&mut keypair_pkey)
        .expect("Cannot read RUST_OPENSTACK_KEYPAIR");
    let floating_network_id = env::var("RUST_OPENSTACK_FLOATING_NETWORK")
        .expect("Missing RUST_OPENSTACK_FLOATING_NETWORK");

    let keypair = os
        .new_keypair("rust-openstack-integration")
        .with_public_key(keypair_pkey)
        .create()
        .expect("Cannot create a key pair");

    let mut port = os
        .new_port(network_id)
        .with_name("rust-openstack-integration")
        .create()
        .expect("Cannot create a port");
    assert_eq!(port.name().as_ref().unwrap(), "rust-openstack-integration");

    let mut server = os
        .new_server("rust-openstack-integration", flavor_id)
        .with_image(image_id)
        .with_port("rust-openstack-integration")
        .with_keypair(keypair)
        .with_metadata("meta", "a3f955c049f7416faa7")
        .create()
        .expect("Failed to request server creation")
        .wait()
        .expect("Server was not created");

    validate_server(&os, &mut server);

    port.refresh().expect("Cannot refresh the port");
    validate_port(&port, &server);

    let network = port.network().expect("Could not find port's network");
    assert_eq!(network.id(), port.network_id());

    let mut floating_ip = os
        .new_floating_ip(floating_network_id)
        .with_port(port.clone())
        .create()
        .expect("Cannot create a floating IP");

    floating_ip.set_description("A floating IP");
    floating_ip.save().expect("Cannot save floating IP");
    assert_eq!(
        floating_ip.description().as_ref().expect("No description"),
        "A floating IP"
    );

    server.refresh().expect("Cannot refresh the server");

    let server_ip = server.floating_ip().expect("No floating IP");
    assert_eq!(server_ip, floating_ip.floating_ip_address());

    floating_ip
        .dissociate()
        .expect("Cannot dissociate a floating IP");

    floating_ip
        .delete()
        .expect("Failed to request floating IP deletion")
        .wait()
        .expect("Failed to delete floating IP");

    server
        .delete()
        .expect("Failed to request deletion")
        .wait()
        .expect("Failed to delete server");

    os.get_keypair("rust-openstack-integration")
        .expect("Cannot get key pair")
        .delete()
        .expect("Cannot delete key pair");

    port.refresh().expect("Cannot refresh the port");
    assert!(port.device_id().is_none());
    assert!(!port.attached_to_server());

    port.delete()
        .expect("Failed to request deletion")
        .wait()
        .expect("Failed to delete port");
}

#[test]
fn test_server_boot_from_new_volume() {
    let os = set_up();
    let image_id = env::var("RUST_OPENSTACK_IMAGE").expect("Missing RUST_OPENSTACK_IMAGE");
    let flavor_id = env::var("RUST_OPENSTACK_FLAVOR").expect("Missing RUST_OPENSTACK_FLAVOR");
    let network_id = env::var("RUST_OPENSTACK_NETWORK").expect("Missing RUST_OPENSTACK_NETWORK");
    let keypair_name = "rust-openstack-integration-bfv";

    let (keypair, private_key) = os
        .new_keypair(keypair_name)
        .with_key_type(openstack::compute::KeyPairType::SSH)
        .generate()
        .expect("Cannot create a key pair");
    assert!(!private_key.is_empty());

    let mut server = os
        .new_server("rust-openstack-integration", flavor_id)
        .with_new_boot_volume(image_id, 8)
        .with_block_device(openstack::compute::BlockDevice::from_empty_volume(8))
        .with_network(network_id.clone())
        .with_keypair(keypair)
        .with_metadata("meta", "a3f955c049f7416faa7")
        .create()
        .expect("Failed to request server creation")
        .wait()
        .expect("Server was not created");

    assert_eq!(server.status(), openstack::compute::ServerStatus::Active);
    assert_eq!(
        server.power_state(),
        openstack::compute::ServerPowerState::Running
    );
    assert!(server.image_id().is_none());

    power_on_off_server(&mut server);

    server
        .delete()
        .expect("Failed to request deletion")
        .wait()
        .expect("Failed to delete server");

    os.get_keypair(keypair_name)
        .expect("Cannot get key pair")
        .delete()
        .expect("Cannot delete key pair");
}
