// Copyright 2019 Dmitry Tantsur <divius.inside@gmail.com>
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

//! JSON structures and protocol bits for the object storage API.

#![allow(missing_docs)]

use osauth::PaginatedResource;
use reqwest::header::{self, HeaderMap, HeaderName};
use serde::Deserialize;

use super::super::common::protocol;
use super::super::{Error, ErrorKind};

#[derive(Debug, Clone, Deserialize)]
pub struct Container {
    pub bytes: u64,
    pub name: String,
    #[serde(rename = "count")]
    pub object_count: u64,
}

impl PaginatedResource for Container {
    type Id = String;
    type Root = Vec<Self>;
    fn resource_id(&self) -> Self::Id {
        self.name.clone()
    }
}

// TODO(dtantsur): implement last_modified. It seems to be complicated by the fact that different
// clouds use different formats (UTC vs naive) or skip it completely (for containers).
#[derive(Debug, Clone, Deserialize)]
pub struct Object {
    pub bytes: u64,
    pub content_type: Option<String>,
    pub name: String,
    pub hash: Option<String>,
}

static CONTENT_LENGTH: HeaderName = header::CONTENT_LENGTH;
static CONTENT_TYPE: HeaderName = header::CONTENT_TYPE;
static ETAG: HeaderName = header::ETAG;

impl PaginatedResource for Object {
    type Id = String;
    type Root = Vec<Self>;
    fn resource_id(&self) -> Self::Id {
        self.name.clone()
    }
}

impl Container {
    pub fn from_headers(name: &str, value: &HeaderMap) -> Result<Container, Error> {
        let bytes_header = HeaderName::from_static("x-container-bytes-used");
        let count_header = HeaderName::from_static("x-container-object-count");
        let bytes: u64 = protocol::get_required_header(value, &bytes_header)?
            .parse()
            .map_err(|e| {
                Error::new(
                    ErrorKind::InvalidResponse,
                    format!("Container-Object-Count is not an integer: {}", e),
                )
            })?;
        let count: u64 = protocol::get_required_header(value, &count_header)?
            .parse()
            .map_err(|e| {
                Error::new(
                    ErrorKind::InvalidResponse,
                    format!("Container-Object-Count is not an integer: {}", e),
                )
            })?;
        Ok(Container {
            bytes,
            name: name.into(),
            object_count: count,
        })
    }
}

impl Object {
    pub fn from_headers(name: &str, value: &HeaderMap) -> Result<Object, Error> {
        let size: u64 = protocol::get_required_header(value, &CONTENT_LENGTH)?
            .parse()
            .map_err(|e| {
                Error::new(
                    ErrorKind::InvalidResponse,
                    format!("ContentLength is not an integer: {}", e),
                )
            })?;
        let ct = protocol::get_header(value, &CONTENT_TYPE)?.map(From::from);
        let hash = protocol::get_header(value, &ETAG)?.map(From::from);
        Ok(Object {
            bytes: size,
            content_type: ct,
            name: name.into(),
            hash: hash,
        })
    }
}
