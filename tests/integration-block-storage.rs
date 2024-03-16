// Copyright 2024 Sandro-Alessio Gierens <sandro@gierens.de>
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

use openstack::block_storage::VolumeStatus;
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
async fn test_volume_create_get_delete_simple() {
    let os = set_up().await;

    let volume = os
        .new_volume(1 as u64)
        .create()
        .await
        .expect("Could not create volume");
    let id = volume.id().clone();
    assert!(volume.name().is_empty());
    assert!(volume.description().is_none());
    assert_eq!(*volume.size(), 1 as u64);

    let volume2 = os.get_volume(&id).await.expect("Could not get volume");
    assert_eq!(volume2.id(), volume.id());

    volume.delete().await.expect("Could not delete volume");

    let volume3 = os.get_volume(id).await;
    assert!(volume3.is_err());
}

#[tokio::test]
async fn test_volume_create_with_fields() {
    let os = set_up().await;

    let volume = os
        .new_volume(1 as u64)
        .with_name("test_volume")
        .with_description("test_description")
        .create()
        .await
        .expect("Could not create volume");
    assert_eq!(volume.name(), "test_volume");
    assert_eq!(*volume.description(), Some("test_description".to_string()));
    assert_eq!(*volume.size(), 1 as u64);

    volume.delete().await.expect("Could not delete volume");
}
