// Copyright 2019 Dmitry Tantsur <divius.inside@gmail.com>
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

use std::future;
use std::sync::Once;
use std::thread;
use std::time::Duration;

use futures::io::Cursor;
use futures::{AsyncReadExt, TryStreamExt};
use openstack::Refresh;

use md5::{Digest, Md5};

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
async fn test_container_create() {
    let os = set_up().await;
    let name = "rust-openstack-integration-empty";

    let ctr = os
        .create_container(name)
        .await
        .expect("Failed to create a container");
    assert_eq!(ctr.name(), name);
    assert_eq!(ctr.bytes(), 0);
    assert_eq!(ctr.object_count(), 0);

    // Duplicate creation succeeds.
    let mut ctr2 = os
        .create_container(name)
        .await
        .expect("Failed to create a duplicate container");
    assert_eq!(ctr2.name(), name);
    assert_eq!(ctr2.bytes(), 0);
    assert_eq!(ctr2.object_count(), 0);

    ctr2.refresh().await.expect("Failed to refresh");
    assert_eq!(ctr2.name(), name);
    assert_eq!(ctr2.bytes(), 0);
    assert_eq!(ctr2.object_count(), 0);

    let found = {
        let stream = os
            .find_containers()
            .into_stream()
            .await
            .expect("Cannot list containers");

        futures::pin_mut!(stream);

        stream
            .try_filter(|ctr| future::ready(ctr.name() == name))
            .try_next()
            .await
    }
    .expect("Failed to list containers")
    .expect("Cannot find the container in listing");
    assert_eq!(found.name(), name);

    let found = os
        .find_containers()
        .with_prefix("rust-")
        .all()
        .await
        .expect("Failed to list containers with prefix");
    assert!(!found.is_empty());

    let found = os
        .find_containers()
        .with_prefix("definitely-not-rust")
        .all()
        .await
        .expect("Failed to list containers with prefix");
    assert!(found.is_empty());

    let objs = ctr.list_objects().await.expect("Failed to list objects");
    assert!(objs.is_empty());

    ctr.delete(false).await.expect("Failed to delete container");

    let found = {
        let stream = os
            .find_containers()
            .into_stream()
            .await
            .expect("Cannot list containers");

        futures::pin_mut!(stream);

        stream
            .try_filter(|ctr| future::ready(ctr.name() == name))
            .try_next()
            .await
    }
    .expect("Failed to list containers");

    assert!(found.is_none());
}

#[tokio::test]
async fn test_object_create() {
    let os = set_up().await;
    let name = "rust-openstack-integration-1";

    let mut ctr = os
        .create_container(name)
        .await
        .expect("Failed to create a container");

    // fake data
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    let buf: Cursor<Vec<u8>> = Cursor::new(data.into());
    // calculate md5 for thos data
    let mut hasher = Md5::new();
    hasher.update(data);
    let data_hash = hasher.finalize();

    let mut obj = os
        .create_object(ctr.clone(), "test1", buf)
        .await
        .expect("Failed to create an object");
    assert_eq!(obj.name(), "test1");
    assert_eq!(obj.container_name(), name);
    assert_eq!(obj.bytes(), 5);
    assert_eq!(obj.hash(), &Some(hex::encode(data_hash)));

    ctr.refresh().await.expect("Failed to refresh container");
    assert!(ctr.object_count() > 0);

    {
        let mut rdr = obj.download().await.expect("Failed to open download");
        let mut res = Vec::new();
        rdr.read_to_end(&mut res)
            .await
            .expect("Failed to read object");
        assert_eq!(res, vec![1, 2, 3, 4, 5]);
    }

    let found = {
        let stream = ctr
            .find_objects()
            .into_stream()
            .await
            .expect("Failed to find objects");

        futures::pin_mut!(stream);

        stream
            .try_filter(|obj| future::ready(obj.name() == "test1"))
            .try_next()
            .await
    }
    .expect("Failed to list objects")
    .expect("Object was not found");
    assert_eq!(found.name(), obj.name());

    obj.refresh().await.expect("Failed to refresh object");

    obj.delete().await.expect("Failed to delete the object");

    let found = {
        let stream = ctr
            .find_objects()
            .into_stream()
            .await
            .expect("Failed to find objects");

        futures::pin_mut!(stream);

        stream
            .try_filter(|obj| future::ready(obj.name() == "test1"))
            .try_next()
            .await
    }
    .expect("Failed to list objects");

    assert!(found.is_none());

    ctr.delete(false)
        .await
        .expect("Failed to delete the container");
}

#[tokio::test]
async fn test_object_with_metadata_and_delete_after() {
    let os = set_up().await;
    let name = "rust-openstack-integration-3";

    let ctr = os
        .create_container(name)
        .await
        .expect("Failed to create a container");

    let buf = Cursor::new(vec![1, 2, 3, 4, 5]);

    os.new_object(ctr.clone(), "test1", buf)
        .with_delete_after(5)
        .with_metadata("answer", "42")
        .create()
        .await
        .expect("Failed to create an object");
    os.get_object(ctr.clone(), "test1")
        .await
        .expect("Failed to fetch object");

    tokio::time::sleep(Duration::from_secs(1)).await;
    let mut maybe_obj = None;

    for _i in 0..60 {
        maybe_obj = os.get_object(ctr.clone(), "test1").await.ok();
        if maybe_obj.is_some() {
            thread::sleep(Duration::from_secs(1));
            maybe_obj = os.get_object(ctr.clone(), "test1").await.ok();
        } else {
            break;
        }
    }
    if let Some(still_obj) = maybe_obj {
        panic!("Object {:?} still exists after 60 seconds", still_obj);
    }

    ctr.delete(true)
        .await
        .expect("Failed to delete the container");
}

#[tokio::test]
async fn test_container_purge() {
    let os = set_up().await;
    let name = "rust-openstack-integration-2";

    let ctr = os
        .create_container(name)
        .await
        .expect("Failed to create a container");

    let buf = Cursor::new(vec![1, 2, 3, 4, 5]);

    let _ = os
        .create_object(ctr.clone(), "test1", buf)
        .await
        .expect("Failed to create an object");

    ctr.delete(true)
        .await
        .expect("Failed to delete the container");

    let found = {
        let stream = os
            .find_containers()
            .into_stream()
            .await
            .expect("Cannot list containers");

        futures::pin_mut!(stream);

        stream
            .try_filter(|ctr| future::ready(ctr.name() == name))
            .try_next()
            .await
    }
    .expect("Failed to list containers");

    assert!(found.is_none());
}
