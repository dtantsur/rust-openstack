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

use std::rc::Rc;
use std::vec;

use fallible_iterator::FallibleIterator;

use super::super::{Error, ErrorKind, Result};
use super::super::session::Session;
use super::super::utils::Query;
use super::{ListResources, ResourceId};


/// Generic implementation of a `FallibleIterator` over resources.
#[derive(Debug, Clone)]
pub struct ResourceIterator<T> {
    session: Rc<Session>,
    query: Query,
    cache: Option<vec::IntoIter<T>>,
    marker: Option<String>,
    can_paginate: Option<bool>,
}

impl<T> ResourceIterator<T> {
    #[allow(dead_code)]  // unused with --no-default-features
    pub(crate) fn new(session: Rc<Session>, query: Query)
            -> ResourceIterator<T> {
        let can_paginate = query.0.iter().all(|pair| {
            pair.0 != "limit" && pair.0 != "marker"
        });

        ResourceIterator {
            session: session,
            query: query,
            cache: None,
            marker: None,
            can_paginate: if can_paginate {
                None  // ask the service later
            } else {
                Some(false)
            }
        }
    }
}

impl<T> ResourceIterator<T> where T: ListResources + ResourceId {
    /// Assert that only one item is left and fetch it.
    ///
    /// Fails with `ResourceNotFound` if no items are left and with
    /// `TooManyItems` if there is more than one item left.
    pub fn one(mut self) -> Result<T> {
        match self.next()? {
            Some(result) => if self.next()?.is_some() {
                Err(Error::new(ErrorKind::TooManyItems,
                               "Query returned more than one result"))
            } else {
                Ok(result)
            },
            None => Err(Error::new(ErrorKind::ResourceNotFound,
                                   "Query returned no results"))
        }
    }
}

impl<T> FallibleIterator for ResourceIterator<T> where T: ListResources + ResourceId {
    type Item = T;

    type Error = Error;

    fn next(&mut self) -> Result<Option<T>> {
        if self.can_paginate.is_none() {
            self.can_paginate = Some(T::can_paginate(&self.session)?);
        }

        let maybe_next = self.cache.as_mut().and_then(|cache| cache.next());
        Ok(if maybe_next.is_some() {
            maybe_next
        } else {
            if self.cache.is_some() && self.can_paginate == Some(false) {
                // We have exhausted the results and pagination is not possible
                None
            } else {
                let mut query = self.query.clone();

                if self.can_paginate == Some(true) {
                    // can_paginate=true implies no limit was provided
                    query.push("limit", T::DEFAULT_LIMIT);
                    if let Some(marker) = self.marker.take() {
                        query.push_str("marker", marker);
                    }
                }

                let mut servers_iter = T::list_resources(self.session.clone(),
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


#[cfg(test)]
mod test {
    use std::rc::Rc;

    use fallible_iterator::FallibleIterator;
    use serde_json::{self, Value};

    use super::super::super::Result;
    use super::super::super::session::Session;
    use super::super::super::utils::{self, Query};
    use super::super::{ListResources, ResourceId};
    use super::ResourceIterator;

    #[derive(Debug, PartialEq, Eq)]
    struct Test(u8);

    impl ResourceId for Test {
        fn resource_id(&self) -> String {
            self.0.to_string()
        }
    }

    fn array_to_map(value: Vec<Value>) -> serde_json::Map<String, Value> {
        value.into_iter().map(|arr| {
           match arr {
               Value::Array(v) => match v[0] {
                   Value::String(ref s) => (s.clone(), v[1].clone()),
                   ref y => panic!("unexpected query key {:?}", y)
               },
               x => panic!("unexpected query component {:?}", x)
           }
        }).collect()
    }

    impl ListResources for Test {
        const DEFAULT_LIMIT: usize = 2;

        fn list_resources<Q>(_session: Rc<Session>, query: Q) -> Result<Vec<Self>>
                where Q: ::serde::Serialize + ::std::fmt::Debug {
            let map = match serde_json::to_value(query).unwrap() {
                Value::Array(arr) => array_to_map(arr),
                x => panic!("unexpected query {:?}", x)
            };
            assert_eq!(*map.get("limit").unwrap(), Value::String("2".into()));
            Ok(match map.get("marker") {
                Some(&Value::String(ref s)) if s == "1" => vec![Test(2), Test(3)],
                Some(&Value::String(ref s)) if s == "3" => Vec::new(),
                None => vec![Test(0), Test(1)],
                Some(ref x) => panic!("unexpected marker {:?}", x)
            })
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct NoPagination(u8);

    impl ListResources for NoPagination {
        const DEFAULT_LIMIT: usize = 2;

        fn can_paginate(_session: &Session) -> Result<bool> { Ok(false) }

        fn list_resources<Q>(_session: Rc<Session>, query: Q) -> Result<Vec<Self>>
                where Q: ::serde::Serialize + ::std::fmt::Debug {
            let map = match serde_json::to_value(query).unwrap() {
                Value::Array(arr) => array_to_map(arr),
                x => panic!("unexpected query {:?}", x)
            };
            assert!(map.get("limit").is_none());
            assert!(map.get("marker").is_none());
            Ok(vec![NoPagination(0), NoPagination(1), NoPagination(2)])
        }
    }

    impl ResourceId for NoPagination {
        fn resource_id(&self) -> String {
            self.0.to_string()
        }
    }

    #[test]
    fn test_resource_iterator() {
        let s = utils::test::new_session(utils::test::URL);
        let it: ResourceIterator<Test> = ResourceIterator::new(Rc::new(s),
                                                               Query::new());
        assert_eq!(it.collect::<Vec<Test>>().unwrap(),
                   vec![Test(0), Test(1), Test(2), Test(3)]);
    }

    #[test]
    fn test_resource_iterator_no_pagination() {
        let s = utils::test::new_session(utils::test::URL);
        let it: ResourceIterator<NoPagination> = ResourceIterator::new(Rc::new(s),
                                                                       Query::new());
        assert_eq!(it.collect::<Vec<NoPagination>>().unwrap(),
                   vec![NoPagination(0), NoPagination(1), NoPagination(2)]);
    }
}
