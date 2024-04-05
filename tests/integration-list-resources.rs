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

use std::sync::Once;

static INIT: Once = Once::new();

async fn set_up() -> openstack::Cloud {
    INIT.call_once(|| {
        env_logger::init();
    });

    openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment")
}

#[tokio::test]
async fn test_list_containers() {
    let os = set_up().await;
    let _ = os.list_containers().await.expect("Cannot list containers");
}

#[tokio::test]
async fn test_list_flavors() {
    let os = set_up().await;
    let items = os.list_flavors().await.expect("Cannot list flavors");
    assert!(!items.is_empty());
}

#[tokio::test]
async fn test_list_images() {
    let os = set_up().await;
    let items = os.list_images().await.expect("Cannot list images");
    assert!(!items.is_empty());
}

#[tokio::test]
async fn test_list_keypairs() {
    let os = set_up().await;
    let _ = os.list_keypairs().await.expect("Cannot list key pairs");
}

#[tokio::test]
async fn test_list_networks() {
    let os = set_up().await;
    let items = os.list_networks().await.expect("Cannot list networks");
    assert!(!items.is_empty());
}

#[tokio::test]
async fn test_list_ports() {
    let os = set_up().await;
    let _ = os.list_ports().await.expect("Cannot list ports");
}

#[tokio::test]
async fn test_list_servers() {
    let os = set_up().await;
    let _ = os.list_servers().await.expect("Cannot list servers");
}

#[tokio::test]
async fn test_list_subnets() {
    let os = set_up().await;
    let _ = os.list_subnets().await.expect("Cannot list subnets");
}

#[tokio::test]
async fn test_list_routers() {
    let os = set_up().await;
    let _ = os.list_routers().await.expect("Cannot list routers");
}

#[tokio::test]
async fn test_list_volumes() {
    let os = set_up().await;
    let _ = os.list_volumes().await.expect("Cannot list volumes");
}
