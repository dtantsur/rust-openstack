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

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use futures::{Stream, TryStreamExt};
use osauth::Query;

use super::{api, infos::*, protocol, types::*};
use crate::{
    common::{ResourceIterator, ResourceQuery},
    session::Session,
    Refresh, Result, Sort,
};

/// Bare metal node - a single physical machine.
#[derive(Debug, Clone)]
pub struct Node {
    session: Session,
    inner: protocol::Node,
}

/// Summary of a bare metal node.
#[derive(Debug, Clone)]
pub struct NodeSummary {
    session: Session,
    inner: protocol::NodeSummary,
}

/// A query to bare metal node list.
#[derive(Clone, Debug)]
pub struct NodeQuery {
    session: Session,
    query: Query<NodeFilter>,
    can_paginate: bool,
}

/// A query to detailed bare metal node list.
#[derive(Clone, Debug)]
pub struct DetailedNodeQuery {
    inner: NodeQuery,
}

#[async_trait]
impl Refresh for Node {
    /// Refresh the node.
    async fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_node(&self.session, &self.inner.id).await?;
        Ok(())
    }
}

impl Node {
    pub(crate) async fn load<Id: AsRef<str>>(session: Session, id_or_name: Id) -> Result<Node> {
        api::get_node(&session, id_or_name)
            .await
            .map(|inner| Node { session, inner })
    }

    transparent_property! {
        /// ID of the allocation claiming this node (if any).
        allocation_id: ref Option<String>
    }

    // TODO(dtantsur): get allocation

    transparent_property! {
        /// Whether automated clean is explicitly enabled or disabled for this node.
        automated_clean: Option<bool>
    }

    transparent_property! {
        /// BIOS interface that the node's driver is using.
        bios_interface: ref String
    }

    transparent_property! {
        /// Boot interface that the node's driver is using.
        boot_interface: ref String
    }

    transparent_property! {
        /// ID of the chassis this node belongs to (if any).
        chassis_id: ref Option<String>
    }

    // TODO(dtantsur): get chassis

    transparent_property! {
        /// Clean step that is currently executed (if any).
        clean_step: ref Option<CleanStep>
    }

    transparent_property! {
        /// Conductor group this node belongs to.
        conductor_group: ref String
    }

    transparent_property! {
        /// The name of the conductor currently responsible for this node.
        ///
        /// This field is actually always populated but only available in recent API versions.
        conductor_name: ref Option<String>
    }

    // TODO(dtantsur): get conductor

    transparent_property! {
        /// Whether serial console is currently enabled for the node.
        console_enabled: bool
    }

    transparent_property! {
        /// Console interface that the node's driver is using.
        console_interface: ref String
    }

    transparent_property! {
        /// When the node was created.
        created_at: DateTime<FixedOffset>
    }

    transparent_property! {
        /// Deploy interface that the node's driver is using.
        deploy_interface: ref String
    }

    transparent_property! {
        /// Deploy step that is currently executed (if any).
        deploy_step: ref Option<DeployStep>
    }

    transparent_property! {
        /// Readable description of the node.
        description: ref Option<String>
    }

    transparent_property! {
        /// The node's driver.
        driver: ref String
    }

    transparent_property! {
        /// Driver-specific configuration.
        driver_info: ref DriverInfo
    }

    transparent_property! {
        /// Operator-provided extra properties.
        extra: ref HashMap<String, serde_json::Value>
    }

    transparent_property! {
        /// Fault that happened on the node.
        fault: Option<Fault>
    }

    transparent_property! {
        /// Unique ID of the node.
        id: ref String
    }

    transparent_property! {
        /// Inspect interface that the node's driver is using.
        inspect_interface: ref String
    }

    transparent_property! {
        /// The date and time when last inspection was finished.
        inspection_finished_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        /// The date and time when last inspection was started.
        inspection_started_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        /// Instance identifier (server ID in case of OpenStack Compute).
        instance_id: ref Option<String>
    }

    transparent_property! {
        /// Instance information specific to the deploy method.
        instance_info: ref InstanceInfo
    }

    transparent_property! {
        /// Last encountered error (cleared on each successful operation).
        last_error: ref Option<String>
    }

    transparent_property! {
        /// The name of a user/project that borrowed this node.
        lessee: ref Option<String>
    }

    transparent_property! {
        /// Whether this node is in maintenance mode.
        maintenance: bool
    }

    transparent_property! {
        /// Reason for maintenance (if provided).
        maintenance_reason: ref Option<String>
    }

    transparent_property! {
        /// Management interface that the node's driver is using.
        management_interface: ref String
    }

    transparent_property! {
        /// Node unique name.
        name: ref Option<String>
    }

    transparent_property! {
        /// Network interface that the node's driver is using.
        network_interface: ref String
    }

    transparent_property! {
        /// The name of a user/project owning this node.
        owner: ref Option<String>
    }

    transparent_property! {
        /// Power interface that the node's driver is using.
        power_interface: ref String
    }

    transparent_property! {
        /// The current power state if the node (if known).
        power_state: Option<PowerState>
    }

    transparent_property! {
        /// Free-form server properties.
        properties: ref Properties
    }

    transparent_property! {
        /// Whether the deployed instance is protected from deletion (undeploy).
        protected: bool
    }

    transparent_property! {
        /// Reason for setting the protected flag (if provided).
        protected_reason: ref Option<String>
    }

    transparent_property! {
        /// The current provision state.
        provision_state: ProvisionState
    }

    transparent_property! {
        /// When the provision state was last updated.
        provision_updated_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        /// RAID interface that the node's driver is using.
        raid_interface: ref String
    }

    transparent_property! {
        /// Rescue interface that the node's driver is using.
        rescue_interface: ref String
    }

    transparent_property! {
        /// Host name of the conductor currently holding a lock on the node.
        reservation: ref Option<String>
    }

    transparent_property! {
        /// Resource class of the node (used for scheduling).
        resource_class: ref Option<String>
    }

    transparent_property! {
        /// Whether the node is marked fo retirement.
        retired: bool
    }

    transparent_property! {
        /// The reason the node was marked for retirement (if provided).
        retired_reason: ref Option<String>
    }

    transparent_property! {
        /// The shard this node belongs to.
        shard: ref Option<String>
    }

    transparent_property! {
        /// Storage interface that the node's driver is using.
        storage_interface: ref String
    }

    transparent_property! {
        /// Target power state (the pending power action).
        target_power_state: Option<TargetPowerState>
    }

    transparent_property! {
        /// Target provision state (the pending provisioning action).
        target_provision_state: Option<TargetProvisionState>
    }

    transparent_property! {
        /// Node traits (used for scheduling).
        traits: ref Vec<String>
    }

    transparent_property! {
        /// When the node was last updated.
        updated_at: Option<DateTime<FixedOffset>>
    }

    transparent_property! {
        /// Vendor interface that the node's driver is using.
        vendor_interface: ref String
    }
}

impl NodeSummary {
    transparent_property! {
        /// Unique ID of the node.
        id: ref String
    }

    transparent_property! {
        /// Instance identifier (server ID in case of OpenStack Compute).
        instance_id: ref Option<String>
    }

    transparent_property! {
        /// Whether this node is in maintenance mode.
        maintenance: bool
    }

    transparent_property! {
        /// Node unique name.
        name: ref Option<String>
    }

    transparent_property! {
        /// The current power state if the node (if known).
        power_state: Option<PowerState>
    }

    transparent_property! {
        /// The current provision state.
        provision_state: ProvisionState
    }
}

impl NodeQuery {
    pub(crate) fn new(session: Session) -> Self {
        Self {
            session,
            query: Query::default(),
            can_paginate: true,
        }
    }

    /// Add a filter to the query.
    pub fn set(&mut self, filter: NodeFilter) {
        if let NodeFilter::Marker(..) | NodeFilter::Limit(..) = filter {
            self.can_paginate = false;
        }
        self.query.push(filter)
    }

    /// Add a filter to the query.
    #[inline]
    pub fn with(mut self, filter: NodeFilter) -> Self {
        self.set(filter);
        self
    }

    /// Add sorting to the request.
    pub fn sort_by(mut self, sort: Sort<NodeSortKey>) -> Self {
        let (field, direction) = sort.unwrap();
        self.query.push(NodeFilter::SortKey(field));
        self.query.push(NodeFilter::SortDir(direction));
        self
    }

    /// Conver this query into a query for detailed nodes.
    #[inline]
    pub fn detailed(self) -> DetailedNodeQuery {
        DetailedNodeQuery { inner: self }
    }

    /// Convert this query into a stream executing the request.
    ///
    /// This stream yields only `NodeSummary` objects, containing the most important
    /// information. Use `detailed().into_stream()` for full `Node` objects.
    ///
    /// Returns a `TryStream`, which is a stream with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    #[inline]
    pub fn into_stream(self) -> impl Stream<Item = Result<NodeSummary>> {
        debug!("Fetching nodes with {:?}", self.query);
        ResourceIterator::new(self).into_stream()
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_stream().try_collect().await`.
    #[inline]
    pub async fn all(self) -> Result<Vec<NodeSummary>> {
        self.into_stream().try_collect().await
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub async fn one(mut self) -> Result<NodeSummary> {
        debug!("Fetching one node with {:?}", self.query);
        if self.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.query.push(NodeFilter::Limit(2))
        }

        ResourceIterator::new(self).one().await
    }

    fn with_marker_and_limit(
        &self,
        limit: Option<usize>,
        marker: Option<String>,
    ) -> Query<NodeFilter> {
        let mut result = self.query.clone();
        if let Some(limit) = limit {
            result.push(NodeFilter::Limit(limit));
        }
        if let Some(marker) = marker {
            result.push(NodeFilter::Marker(marker));
        }
        result
    }
}

#[async_trait]
impl ResourceQuery for NodeQuery {
    type Item = NodeSummary;

    const DEFAULT_LIMIT: usize = 100;

    async fn can_paginate(&self) -> Result<bool> {
        Ok(self.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.id().clone()
    }

    async fn fetch_chunk(
        &self,
        limit: Option<usize>,
        marker: Option<String>,
    ) -> Result<Vec<Self::Item>> {
        let query = self.with_marker_and_limit(limit, marker);
        Ok(api::list_nodes(&self.session, &query)
            .await?
            .into_iter()
            .map(|srv| NodeSummary {
                session: self.session.clone(),
                inner: srv,
            })
            .collect())
    }
}

impl DetailedNodeQuery {
    /// Add a filter to the query.
    pub fn set(&mut self, filter: NodeFilter) {
        self.inner.set(filter);
    }

    /// Add a filter to the query.
    #[inline]
    pub fn with(mut self, filter: NodeFilter) -> Self {
        self.inner.set(filter);
        self
    }

    /// Add sorting to the request.
    pub fn sort_by(self, sort: Sort<NodeSortKey>) -> Self {
        Self {
            inner: self.inner.sort_by(sort),
        }
    }

    /// Convert this query into a stream executing the request.
    ///
    /// Returns a `TryStream`, which is a stream with each `next`
    /// call returning a `Result`.
    ///
    /// Note that no requests are done until you start iterating.
    #[inline]
    pub fn into_stream(self) -> impl Stream<Item = Result<Node>> {
        debug!("Fetching nodes with {:?}", self.inner.query);
        ResourceIterator::new(self).into_stream()
    }

    /// Execute this request and return all results.
    ///
    /// A convenience shortcut for `self.into_stream().try_collect().await`.
    #[inline]
    pub async fn all(self) -> Result<Vec<Node>> {
        self.into_stream().try_collect().await
    }

    /// Return one and exactly one result.
    ///
    /// Fails with `ResourceNotFound` if the query produces no results and
    /// with `TooManyItems` if the query produces more than one result.
    pub async fn one(mut self) -> Result<Node> {
        debug!("Fetching one node with {:?}", self.inner.query);
        if self.inner.can_paginate {
            // We need only one result. We fetch maximum two to be able
            // to check if the query yieled more than one result.
            self.inner.query.push(NodeFilter::Limit(2))
        }

        ResourceIterator::new(self).one().await
    }

    fn with_marker_and_limit(
        &self,
        limit: Option<usize>,
        marker: Option<String>,
    ) -> Query<NodeFilter> {
        let mut result = self.inner.query.clone();
        if let Some(limit) = limit {
            result.push(NodeFilter::Limit(limit));
        }
        if let Some(marker) = marker {
            result.push(NodeFilter::Marker(marker));
        }
        result
    }
}

#[async_trait]
impl ResourceQuery for DetailedNodeQuery {
    type Item = Node;

    const DEFAULT_LIMIT: usize = 100;

    async fn can_paginate(&self) -> Result<bool> {
        Ok(self.inner.can_paginate)
    }

    fn extract_marker(&self, resource: &Self::Item) -> String {
        resource.id().clone()
    }

    async fn fetch_chunk(
        &self,
        limit: Option<usize>,
        marker: Option<String>,
    ) -> Result<Vec<Self::Item>> {
        let query = self.with_marker_and_limit(limit, marker);
        Ok(api::list_nodes_detailed(&self.inner.session, &query)
            .await?
            .into_iter()
            .map(|srv| Node {
                session: self.inner.session.clone(),
                inner: srv,
            })
            .collect())
    }
}

impl NodeSummary {
    /// Get details.
    pub async fn details(&self) -> Result<Node> {
        Node::load(self.session.clone(), &self.inner.id).await
    }
}
