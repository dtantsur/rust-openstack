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

//! Adapters between entity types.


/// Trait for something that can be used as an image ID.
pub trait ToImageId {
    /// Get flavor ID as a string.
    fn to_image_id(&self) -> String;
}

/// Trait for something that can be used as a flavor ID.
pub trait ToFlavorId {
    /// Get flavor ID as a string.
    fn to_flavor_id(&self) -> String;
}


impl ToImageId for String {
    fn to_image_id(&self) -> String {
        self.clone()
    }
}

impl ToImageId for str {
    fn to_image_id(&self) -> String {
        String::from(self)
    }
}

impl ToFlavorId for String {
    fn to_flavor_id(&self) -> String {
        self.clone()
    }
}

impl ToFlavorId for str {
    fn to_flavor_id(&self) -> String {
        String::from(self)
    }
}
