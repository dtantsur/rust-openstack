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

use crate::common::CommaSeparated;
use crate::utils::SortDir;
use osauth::QueryItem;
use serde::Deserialize;

protocol_enum! {
    /// Provision state of the node.
    enum ProvisionState = Unknown {
        /// Previously deployed node is being adopted.
        Adopting = "adopting",
        /// Adopting a deployed node has failed.
        AdoptFailed = "adopt failed",
        /// Node is deployed.
        Active = "active",
        /// Node is available for deployment.
        Available = "available",
        /// A synchronous cleaning/preparing action is running.
        Cleaning = "cleaning",
        /// Cleaning has failed.
        CleanFailed = "clean failed",
        /// Waiting for an asynchronous cleaning/preparing action.
        CleanWait = "clean wait",
        /// A synchronous deployment action is running.
        Deploying = "deploying",
        /// Deployment has failed.
        DeployFailed = "deploy failed",
        /// Waiting for an asynchronous deployment action.
        DeployWait = "wait call-back",
        /// Processing inspection data.
        Inspecting = "inspecting",
        /// Inspection has failed.
        InspectFailed = "inspect failed",
        /// Waiting for inspection data from the ramdisk.
        InspectWait = "inspect wait",
        /// Node is freshly enrolled.
        Enroll = "enroll",
        /// Node is enrolled and manageable.
        Manageable = "manageable",
        /// Node is in rescue mode.
        Rescue = "rescue",
        /// Node is being prepared for rescue.
        Rescuing = "rescuing",
        /// Rescuing node failed.
        RescueFailed = "rescue failed",
        /// Waiting for rescue ramdisk to come up.
        RescueWait = "rescue wait",
        /// Node is being undeployed (instance deletion).
        Undeploying = "deleting",
        /// Undeployment failed before cleaning.
        UndeployFailed = "error",
        /// Node is exiting rescue mode.
        Unrescuing = "unrescuing",
        /// Exiting rescue mode has failed.
        UnrescueFailed = "unrescue failed",
        /// Management access is being verified.
        Verifying = "verifying",

        /// Reported provision state is not supported.
        Unknown = ""
    }
}

impl ProvisionState {
    /// Whether the state is stable.
    ///
    /// A node will stay in a stable state forever, unless explicitly moved to a different state.
    /// Error states are not considered stable since they require an action.
    pub fn is_stable(&self) -> bool {
        matches!(
            self,
            ProvisionState::Active
                | ProvisionState::Available
                | ProvisionState::Enroll
                | ProvisionState::Manageable
                | ProvisionState::Rescue
        )
    }

    /// Whether the state represents a failure.
    ///
    /// Failure states are similar to stable states since nodes do not leave them automatically.
    /// But they require intervention for recovery.
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            ProvisionState::AdoptFailed
                | ProvisionState::CleanFailed
                | ProvisionState::DeployFailed
                | ProvisionState::InspectFailed
                | ProvisionState::RescueFailed
                | ProvisionState::UndeployFailed
                | ProvisionState::UnrescueFailed
        )
    }
}

protocol_enum! {
    /// Target provision state of the node.
    enum TargetProvisionState {
        /// Node will be deployed (instance active).
        Active = "active",
        /// Node will be undeployed (instance deleted).
        Deleted = "deleted",
        /// Node will be available (after instance deletion and cleaning).
        Available = "available",
        /// Node will be manageable.
        Manageable = "manageable",
        /// Node will be in rescue mode.
        Rescue = "rescue"
    }
}

protocol_enum! {
    /// Power state of the node.
    enum PowerState {
        /// Node is powered off.
        Off = "power off",
        /// Node is powered on.
        On = "power on",
        /// Error when getting power state.
        Error = "error"
    }
}

protocol_enum! {
    /// Target power state of the node.
    enum TargetPowerState {
        /// Power off the node (hard power off).
        Off = "power off",
        /// Power on the node.
        On = "power on",
        /// Reboot the node (hard reboot).
        Reboot = "rebooting",
        /// Power off the node (soft power off).
        SoftOff = "soft power off",
        /// Reboot the node (soft reboot).
        SoftReboot = "soft rebooting"
    }
}

protocol_enum! {
    /// Interface of a deploy or clean step
    enum StepInterface {
        BIOS = "bios",
        Deploy = "deploy",
        Management = "management",
        Power = "power",
        RAID = "raid"
    }
}

protocol_enum! {
    /// Type of a fault.
    enum Fault {
        /// Failure to manage the power state.
        Power = "power failure",
        /// Failure of a clean step.
        Clean = "clean failure",
        /// Failure to clean up when aborting rescue.
        RescueAbort = "rescue abort failure"
    }
}

#[derive(Debug, Clone, Deserialize)]
/// A deploy step.
pub struct DeployStep {
    /// Interface to which the step belongs.
    pub interface: StepInterface,
    /// Step name.
    #[serde(rename = "step")]
    pub name: String,
    /// Priority in which the step runs.
    pub priority: u32,
}

#[derive(Debug, Clone, Deserialize)]
/// A clean step.
pub struct CleanStep {
    /// Whether cleaning can be aborted on this step.
    #[serde(default)]
    pub abortable: bool,
    /// Interface to which the step belongs.
    pub interface: StepInterface,
    /// Step name.
    #[serde(rename = "step")]
    pub name: String,
    /// Priority in which the step runs.
    pub priority: u32,
    /// Whether the step requires an agent ramdisk to be running.
    #[serde(default = "crate::utils::some_truth")]
    pub requires_ramdisk: bool,
}

protocol_enum! {
    /// Sort key for listing nodes.
    #[allow(missing_docs)]
    enum NodeSortKey {
        AllocationID = "allocation_uuid",
        AutomatedClean = "automated_clean",
        BIOSInterface = "bios_interface",
        BootInterface = "boot_interface",
        ChassisID = "chassis_uuid",
        ConductorGroup = "conductor_group",
        // TODO(dtantsur): is sorting by conductor actually possible?
        // ConductorName = "conductor",
        ConsoleEnabled = "console_enabled",
        ConsoleInterface = "console_interface",
        CreatedAt = "created_at",
        DeployInterface = "deploy_interface",
        Description = "description",
        Driver = "driver",
        ID = "uuid",
        InspectInterface = "inspect_interface",
        InspectionFinishedAt = "inspection_finished_at",
        InspectionStartedAt = "inspection_started_at",
        InstanceID = "instance_uuid",
        Lessee = "lessee",
        Maintenance = "maintenance",
        ManagementInterface = "management_interface",
        Name = "name",
        NetworkInterface = "network_interface",
        Owner = "owner",
        PowerInterface = "power_interface",
        PowerState = "power_state",
        Protected = "protected",
        ProvisionState = "provision_state",
        ProvisionUpdatedAt = "provision_updated_at",
        RAIDInterface = "raid_interface",
        RescueInterface = "rescue_interface",
        Reservation = "reservation",
        ResourceClass = "resource_class",
        Retired = "retired",
        Shard = "shard",
        StorageInterface = "storage_interface",
        TargetPowerState = "target_power_state",
        TargetProvisionState = "target_provision_state",
        UpdatedAt = "updated_at",
        VendorInterface = "vendor_interface"
    }
}

/// Filter for node objects.
#[derive(Debug, Clone, QueryItem)]
pub enum NodeFilter {
    // FIXME(dtantsur): Marker and Limit are always used, move out of Vec<NodeFilter> (in Query)
    /// Marker (last Node that was fetched).
    Marker(String),
    /// Limit on the number of fetched nodes.
    Limit(usize),
    /// Key to sort on.
    SortKey(NodeSortKey),
    /// Sorting direction.
    SortDir(SortDir),

    /// Node associated with an instance.
    Associated(bool),
    /// Nodes with the given chassis UUID.
    ChassisID(String),
    /// Nodes with descriptions containing this string.
    DescriptionContains(String),
    /// Nodes that belong to this conductor group.
    ConductorGroup(String),
    /// Nodes with this driver.
    Driver(String),
    /// Nodes that have a fault of this type.
    Fault(String),
    /// Include nodes with a parent node.
    IncludeChildren(bool),
    /// Nodes leased by this project or user ID.
    Lessee(String),
    /// Nodes in or not in maintenance mode.
    Maintenance(bool),
    /// Nodes owned by this project or user ID.
    Owner(String),
    /// Nodes that a children of the given node.
    ParentNode(String),
    /// Nodes owned by this project ID.
    Project(String),
    /// Nodes in the given provision state.
    ProvisionState(ProvisionState),
    /// Nodes with this resource class.
    ResourceClass(String),
    /// Nodes that are retired.
    Retired(bool),
    /// Nodes that have the shard field populated.
    Sharded(bool),
    /// Nodes that belong to one of these shards.
    ShardIn(CommaSeparated<String>),
}

impl NodeFilter {
    /// Helper for ShardIn.
    pub fn shard_in<I>(shards: I) -> NodeFilter
    where
        I: IntoIterator,
        String: From<I::Item>,
    {
        NodeFilter::ShardIn(CommaSeparated(shards.into_iter().map(From::from).collect()))
    }
}
