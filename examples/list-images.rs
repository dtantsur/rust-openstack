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

extern crate env_logger;
extern crate fallible_iterator;
extern crate openstack;

use fallible_iterator::FallibleIterator;


#[cfg(feature = "image")]
fn main() {
    env_logger::init();

    let identity = openstack::auth::from_env()
        .expect("Failed to create an identity provider from the environment");
    let os = openstack::Cloud::new(identity);

    let images: Vec<openstack::image::Image> = os.find_images()
        .sort_by(openstack::Sort::Asc(openstack::image::ImageSortKey::Id))
        .into_iter().take(10).collect()
        .expect("Cannot list images");
    println!("First 10 images:");
    for img in &images {
        println!("ID = {}, Name = {}, Status = {}, Visibility = {}",
                 img.id(), img.name(), img.status(), img.visibility());
    }

    let mut public = os.find_images()
        .sort_by(openstack::Sort::Asc(openstack::image::ImageSortKey::Name))
        .with_visibility(openstack::image::ImageVisibility::Public)
        .into_iter();
    println!("All public images:");
    while let Some(img) = public.next().unwrap() {
        println!("ID = {}, Name = {}, Status = {}, Visibility = {}",
                 img.id(), img.name(), img.status(), img.visibility());
    }
}

#[cfg(not(feature = "image"))]
fn main() {
    panic!("This example cannot run with 'image' feature disabled");
}
