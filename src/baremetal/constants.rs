// Copyright 2023 Dmitry Tantsur <dtantsur@protonmail.com>
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

use osauth::ApiVersion;

pub const API_VERSION_MINIMUM: ApiVersion = ApiVersion(1, 46); // Rocky
pub const API_VERSION_AUTOMATED_CLEAN: ApiVersion = ApiVersion(1, 47);
pub const API_VERSION_PROTECTED: ApiVersion = ApiVersion(1, 48);
pub const API_VERSION_CONDUCTORS: ApiVersion = ApiVersion(1, 49);
pub const API_VERSION_OWNER: ApiVersion = ApiVersion(1, 50);
pub const API_VERSION_DESCRIPTION: ApiVersion = ApiVersion(1, 51);
pub const API_VERSION_ALLOCATIONS: ApiVersion = ApiVersion(1, 52);
pub const API_VERSION_RETIRED: ApiVersion = ApiVersion(1, 61);
pub const API_VERSION_LESSEE: ApiVersion = ApiVersion(1, 65);
pub const API_VERSION_NETWORK_DATA: ApiVersion = ApiVersion(1, 66);
pub const API_VERSION_BOOT_MODE: ApiVersion = ApiVersion(1, 75);
pub const API_VERSION_SHARDS: ApiVersion = ApiVersion(1, 82);
pub const API_VERSION_CHILD_NODES: ApiVersion = ApiVersion(1, 83);
