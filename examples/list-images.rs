// Copyright 2017 Dmitry Tantsur <divius.inside@gmail.com>
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

use futures::pin_mut;
use futures::stream::{StreamExt, TryStreamExt};

#[cfg(feature = "image")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();

    let os = openstack::Cloud::from_env()
        .await
        .expect("Failed to create an identity provider from the environment");

    let images: Vec<openstack::image::Image> = os
        .find_images()
        .sort_by(openstack::Sort::Asc(openstack::image::ImageSortKey::Id))
        .into_stream()
        .take(10)
        .try_collect()
        .await
        .expect("Cannot list images");
    println!("First 10 images:");
    for img in &images {
        println!(
            "ID = {}, Name = {}, Status = {}, Visibility = {}",
            img.id(),
            img.name(),
            img.status(),
            img.visibility()
        );
    }

    let public = os
        .find_images()
        .sort_by(openstack::Sort::Asc(openstack::image::ImageSortKey::Name))
        .with_visibility(openstack::image::ImageVisibility::Public)
        .into_stream();
    println!("All public images:");
    pin_mut!(public);
    while let Some(img) = public.next().await.transpose().unwrap() {
        println!(
            "ID = {}, Name = {}, Status = {}, Visibility = {}",
            img.id(),
            img.name(),
            img.status(),
            img.visibility()
        );
    }
}

#[cfg(not(feature = "image"))]
fn main() {
    panic!("This example cannot run with 'image' feature disabled");
}
