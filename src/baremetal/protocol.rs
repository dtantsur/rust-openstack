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

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::infos::{DriverInfo, InstanceInfo, Properties};
use super::types::*;
use crate::utils::{empty_map_as_default, is_default};

#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    #[serde(default, rename = "allocation_uuid")]
    pub allocation_id: Option<String>,
    #[serde(default)]
    pub automated_clean: Option<bool>,
    pub bios_interface: String,
    pub boot_interface: String,
    // TODO(dtantsur): boot_mode/secure_boot
    #[serde(default, rename = "chassis_uuid")]
    pub chassis_id: Option<String>,
    #[serde(default, deserialize_with = "empty_map_as_default")]
    pub clean_step: Option<CleanStep>,
    #[serde(default, rename = "conductor")]
    pub conductor_name: Option<String>,
    pub conductor_group: String,
    pub console_enabled: bool,
    pub console_interface: String,
    pub created_at: DateTime<FixedOffset>,
    pub deploy_interface: String,
    #[serde(default, deserialize_with = "empty_map_as_default")]
    pub deploy_step: Option<DeployStep>,
    #[serde(default)]
    pub description: Option<String>,
    pub driver: String,
    pub driver_info: DriverInfo,
    pub extra: HashMap<String, Value>,
    #[serde(default)]
    pub fault: Option<Fault>,
    #[serde(rename = "uuid")]
    pub id: String,
    pub inspect_interface: String,
    #[serde(default)]
    pub inspection_finished_at: Option<DateTime<FixedOffset>>,
    #[serde(default)]
    pub inspection_started_at: Option<DateTime<FixedOffset>>,
    #[serde(rename = "instance_uuid")]
    pub instance_id: Option<String>,
    pub instance_info: InstanceInfo,
    pub last_error: Option<String>,
    #[serde(default)]
    pub lessee: Option<String>,
    pub maintenance: bool,
    #[serde(default)]
    pub maintenance_reason: Option<String>,
    pub management_interface: String,
    #[serde(default)]
    pub name: Option<String>,
    // #[serde(default, deserialize_with = "empty_map_as_default")]
    // pub network_data: Option<Value>,
    pub network_interface: String,
    #[serde(default)]
    pub owner: Option<String>,
    pub power_interface: String,
    #[serde(default)]
    pub power_state: Option<PowerState>,
    pub properties: Properties,
    #[serde(default)]
    pub protected: bool,
    #[serde(default)]
    pub protected_reason: Option<String>,
    pub provision_state: ProvisionState,
    pub provision_updated_at: Option<DateTime<FixedOffset>>,
    // TODO(dtantsur): raid_config
    pub raid_interface: String,
    pub rescue_interface: String,
    #[serde(default)]
    pub reservation: Option<String>,
    #[serde(default)]
    pub resource_class: Option<String>,
    #[serde(default)]
    pub retired: bool,
    #[serde(default)]
    pub retired_reason: Option<String>,
    #[serde(default)]
    pub shard: Option<String>,
    pub storage_interface: String,
    #[serde(default)]
    pub target_power_state: Option<TargetPowerState>,
    #[serde(default)]
    pub target_provision_state: Option<TargetProvisionState>,
    #[serde(default)]
    pub traits: Vec<String>,
    #[serde(default)]
    pub updated_at: Option<DateTime<FixedOffset>>,
    pub vendor_interface: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeSummary {
    #[serde(rename = "uuid")]
    pub id: String,
    #[serde(default, rename = "instance_uuid")]
    pub instance_id: Option<String>,
    pub maintenance: bool,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub power_state: Option<PowerState>,
    pub provision_state: ProvisionState,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodesDetailRoot {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodesRoot {
    pub nodes: Vec<NodeSummary>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[allow(dead_code)] // FIXME(dtantsur): remove when creating is implemented
pub struct NodeCreate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automated_clean: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bios_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chassis_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deploy_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub driver: String,
    pub driver_info: DriverInfo,
    pub extra: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inspect_interface: Option<String>,
    pub instance_info: InstanceInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lessee: Option<String>,
    #[serde(skip_serializing_if = "is_default")]
    pub maintenance: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub management_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_interface: Option<String>,
    pub properties: Properties,
    #[serde(skip_serializing_if = "is_default")]
    pub protected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rescue_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_class: Option<String>,
    #[serde(skip_serializing_if = "is_default")]
    pub retired: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retired_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_interface: Option<String>,
}

#[cfg(test)]
mod test {
    mod node {
        use serde_json::json;

        use super::super::*;

        #[test]
        fn test_base_version() {
            let node_json = json!({
                "uuid": "abcd",
                "chassis_uuid": None::<String>,
                "conductor_group": "",
                "console_enabled": false,
                "created_at": "2016-08-18T22:28:48.643434+11:11",
                "driver": "ipmi",
                "driver_info": {
                    "ipmi_address": "1.2.3.4"
                },
                "extra": {},
                "instance_info": {},
                "last_error": None::<String>,
                "maintenance": false,
                "power_state": "power off",
                "properties": {
                    "root_device": {
                        "name": "/dev/sda"
                    }
                },
                "provision_state": "available",

                "bios_interface": "no-bios",
                "boot_interface": "ipxe",
                "console_interface": "no-console",
                "deploy_interface": "direct",
                "inspect_interface": "inspector",
                "management_interface": "ipmitool",
                "network_interface": "neutron",
                "power_interface": "ipmitool",
                "raid_interface": "agent",
                "rescue_interface": "agent",
                "storage_interface": "noop",
                "vendor_interface": "no-vendor"
            });

            let node: Node = serde_json::from_value(node_json).unwrap();
            assert_eq!(&node.id, "abcd");
            assert_eq!(node.provision_state, ProvisionState::Available);
            assert_eq!(node.power_state.unwrap(), PowerState::Off);
            assert_eq!(
                *node.driver_info.get("ipmi_address").unwrap(),
                serde_json::Value::String("1.2.3.4".into())
            );
            assert!(node.last_error.is_none());
            assert!(node.extra.is_empty());
        }

        #[test]
        fn test_steps() {
            let node_json = json!({
                "uuid": "abcd",
                "chassis_uuid": None::<String>,
                "conductor_group": "",
                "console_enabled": false,
                "created_at": "2016-08-18T22:28:48.643434+11:11",
                "driver": "ipmi",
                "driver_info": {
                    "ipmi_address": "1.2.3.4"
                },
                "extra": {},
                "instance_info": {},
                "last_error": None::<String>,
                "maintenance": false,
                "power_state": "power off",
                "properties": {
                    "root_device": {
                        "name": "/dev/sda"
                    }
                },
                "provision_state": "available",

                "bios_interface": "no-bios",
                "boot_interface": "ipxe",
                "console_interface": "no-console",
                "deploy_interface": "direct",
                "inspect_interface": "inspector",
                "management_interface": "ipmitool",
                "network_interface": "neutron",
                "power_interface": "ipmitool",
                "raid_interface": "agent",
                "rescue_interface": "agent",
                "storage_interface": "noop",
                "vendor_interface": "no-vendor",

                "clean_step": {
                    "step": "write_image",
                    "interface": "deploy",
                    "priority": 50
                },
                "deploy_step": {},
            });

            let node: Node = serde_json::from_value(node_json).unwrap();
            assert!(node.deploy_step.is_none());
            assert_eq!(node.clean_step.as_ref().unwrap().name, "write_image");
            assert_eq!(
                node.clean_step.as_ref().unwrap().interface,
                StepInterface::Deploy
            );
        }
    }
}
