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

//! Volume management via Block Storage API.

use async_trait::async_trait;
use futures::stream::{Stream, TryStreamExt};

use super::super::common::{Refresh, ResourceIterator, ResourceQuery};
use super::super::session::Session;
use super::super::utils::Query;
use super::super::{Result, Sort};
use super::{api, protocol};

/// Structure representing a summary of a single volume.
#[derive(Clone, Debug)]
pub struct Volume {
    session: Session,
    inner: protocol::Volume,
}

impl Volume {
    /// Create an Volume object.
    pub(crate) async fn new<Id: AsRef<str>>(session: Session, id: Id) -> Result<Volume> {
        let inner = api::get_volume(&session, id).await?;
        Ok(Volume { session, inner })
    }

    transparent_property! {
        #[doc = "Unique ID."]
        id: ref String
    }

    transparent_property! {
        #[doc = "Volume name."]
        name: ref String
    }
}

#[async_trait]
impl Refresh for Volume {
    /// Refresh the volume.
    async fn refresh(&mut self) -> Result<()> {
        self.inner = api::get_volume_by_id(&self.session, &self.inner.id).await?;
        Ok(())
    }
}
