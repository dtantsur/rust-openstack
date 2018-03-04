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

//! Generic API bits for implementing new services.

use std::vec;

use fallible_iterator::FallibleIterator;

use super::super::{Error, Result};
use super::super::session::Session;
use super::super::utils::Query;
use super::{ListResources, ResourceId};


/// Generic implementation of a `FallibleIterator` over resources.
#[derive(Debug, Clone)]
pub struct ResourceIterator<'session, T> {
    session: &'session Session,
    query: Query,
    cache: Option<vec::IntoIter<T>>,
    marker: Option<String>,
    can_paginate: bool,
}

impl<'session, T> ResourceIterator<'session, T> {
    #[allow(dead_code)]  // unused with --no-default-features
    pub(crate) fn new(session: &'session Session, query: Query)
            -> ResourceIterator<'session, T> {
        let can_paginate = query.0.iter().all(|pair| {
            pair.0 != "limit" && pair.0 != "marker"
        });

        ResourceIterator {
            session: session,
            query: query,
            cache: None,
            marker: None,
            can_paginate: can_paginate
        }
    }
}

impl<'session, T> FallibleIterator for ResourceIterator<'session, T>
        where T: ListResources<'session> + ResourceId {
    type Item = T;

    type Error = Error;

    fn next(&mut self) -> Result<Option<T>> {
        let maybe_next = self.cache.as_mut().and_then(|cache| cache.next());
        Ok(if maybe_next.is_some() {
            maybe_next
        } else {
            if self.cache.is_some() && ! self.can_paginate {
                // We have exhausted the results and pagination is not possible
                None
            } else {
                let mut query = self.query.clone();

                if self.can_paginate {
                    // can_paginate=true implies no limit was provided
                    query.push("limit", T::DEFAULT_LIMIT);
                    if let Some(marker) = self.marker.take() {
                        query.push_str("marker", marker);
                    }
                }

                let mut servers_iter = T::list_resources(self.session,
                                                         &query.0)?
                    .into_iter();
                let maybe_next = servers_iter.next();
                self.cache = Some(servers_iter);

                maybe_next
            }
        }.map(|next| {
            self.marker = Some(next.resource_id());
            next
        }))
    }
}
