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

use super::super::{Error, ErrorKind, Result};

/// A query for resources.
///
/// This is a low-level trait that should not be used directly.
pub trait ResourceQuery {
    /// Item type.
    type Item;

    /// Default limit to use with this query.
    const DEFAULT_LIMIT: usize;

    /// Whether pagination is supported for this query.
    fn can_paginate(&self) -> Result<bool>;

    /// Extract a marker from a resource.
    fn extract_marker(&self, resource: &Self::Item) -> String;

    /// Get a chunk of resources.
    fn fetch_chunk(&self, limit: Option<usize>, marker: Option<String>) -> Result<Vec<Self::Item>>;

    /// Validate the query before the first execution.
    ///
    /// This call may modify internal representation of the query, so changing
    /// the query after calling it may cause undesired side effects.
    fn validate(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Generic implementation of a `FallibleIterator` over resources.
#[derive(Debug, Clone)]
pub struct ResourceIterator<Q: ResourceQuery> {
    query: Q,
    cache: Option<vec::IntoIter<Q::Item>>,
    marker: Option<String>,
    can_paginate: Option<bool>,
    validated: bool,
}

impl<Q> ResourceIterator<Q>
where
    Q: ResourceQuery,
{
    #[allow(dead_code)] // unused with --no-default-features
    pub(crate) fn new(query: Q) -> ResourceIterator<Q> {
        ResourceIterator {
            query,
            cache: None,
            marker: None,
            can_paginate: None, // ask the service later
            validated: false,
        }
    }

    /// Assert that only one item is left and fetch it.
    ///
    /// Fails with `ResourceNotFound` if no items are left and with
    /// `TooManyItems` if there is more than one item left.
    pub fn one(mut self) -> Result<Q::Item> {
        match self.next()? {
            Some(result) => {
                if self.next()?.is_some() {
                    Err(Error::new(
                        ErrorKind::TooManyItems,
                        "Query returned more than one result",
                    ))
                } else {
                    Ok(result)
                }
            }
            None => Err(Error::new(
                ErrorKind::ResourceNotFound,
                "Query returned no results",
            )),
        }
    }
}

impl<Q> FallibleIterator for ResourceIterator<Q>
where
    Q: ResourceQuery,
{
    type Item = Q::Item;

    type Error = Error;

    fn next(&mut self) -> Result<Option<Self::Item>> {
        if !self.validated {
            self.query.validate()?;
            self.validated = true;
        }

        if self.can_paginate.is_none() {
            self.can_paginate = Some(self.query.can_paginate()?);
        }

        let maybe_next = self.cache.as_mut().and_then(|cache| cache.next());
        Ok(if maybe_next.is_some() {
            maybe_next
        } else if self.cache.is_some() && self.can_paginate == Some(false) {
            // We have exhausted the results and pagination is not possible
            None
        } else {
            let (marker, limit) = if self.can_paginate == Some(true) {
                // can_paginate=true implies no limit was provided
                (self.marker.clone(), Some(Q::DEFAULT_LIMIT))
            } else {
                (None, None)
            };

            let mut iter = self.query.fetch_chunk(limit, marker)?.into_iter();
            let maybe_next = iter.next();
            self.cache = Some(iter);

            maybe_next
        }
        .map(|next| {
            self.marker = Some(self.query.extract_marker(&next));
            next
        }))
    }
}

#[cfg(test)]
mod test {
    use fallible_iterator::FallibleIterator;

    use super::super::super::Result;
    use super::{ResourceIterator, ResourceQuery};

    #[derive(Debug, PartialEq, Eq)]
    struct Test(u8);

    #[derive(Debug)]
    struct TestQuery;

    impl ResourceQuery for TestQuery {
        type Item = Test;

        const DEFAULT_LIMIT: usize = 2;

        fn can_paginate(&self) -> Result<bool> {
            Ok(true)
        }

        fn extract_marker(&self, resource: &Test) -> String {
            resource.0.to_string()
        }

        fn fetch_chunk(
            &self,
            limit: Option<usize>,
            marker: Option<String>,
        ) -> Result<Vec<Self::Item>> {
            assert_eq!(limit, Some(2));
            Ok(match marker.map(|s| s.parse::<u8>().unwrap()) {
                Some(1) => vec![Test(2), Test(3)],
                Some(3) => Vec::new(),
                None => vec![Test(0), Test(1)],
                Some(x) => panic!("unexpected marker {:?}", x),
            })
        }
    }

    #[derive(Debug)]
    struct NoPagination;

    impl ResourceQuery for NoPagination {
        type Item = Test;

        const DEFAULT_LIMIT: usize = 2;

        fn can_paginate(&self) -> Result<bool> {
            Ok(false)
        }

        fn extract_marker(&self, resource: &Test) -> String {
            resource.0.to_string()
        }

        fn fetch_chunk(
            &self,
            limit: Option<usize>,
            marker: Option<String>,
        ) -> Result<Vec<Self::Item>> {
            assert!(limit.is_none());
            assert!(marker.is_none());
            Ok(vec![Test(0), Test(1), Test(2)])
        }
    }

    #[test]
    fn test_resource_iterator() {
        let it: ResourceIterator<TestQuery> = ResourceIterator::new(TestQuery);
        assert_eq!(
            it.collect::<Vec<Test>>().unwrap(),
            vec![Test(0), Test(1), Test(2), Test(3)]
        );
    }

    #[test]
    fn test_resource_iterator_no_pagination() {
        let it: ResourceIterator<NoPagination> = ResourceIterator::new(NoPagination);
        assert_eq!(
            it.collect::<Vec<Test>>().unwrap(),
            vec![Test(0), Test(1), Test(2)]
        );
    }
}
