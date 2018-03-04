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

//! ApiVersion implementation.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error as DeserError, Visitor};

use super::super::{Error, ErrorKind, Result};


/// API version (major, minor).
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct ApiVersion(pub u16, pub u16);

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

fn parse_component(component: &str, message: &str) -> Result<u16> {
    component.parse().map_err(|_| {
        Error::new(ErrorKind::InvalidResponse, message)
    })
}

impl FromStr for ApiVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<ApiVersion> {
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() != 2 {
            let msg = format!("Invalid API version: expected X.Y, got {}", s);
            return Err(Error::new(ErrorKind::InvalidResponse, msg))
        }

        let major = parse_component(parts[0],
                                    "First version component is not a number")?;

        let minor = parse_component(parts[1],
                                    "Second version component is not a number")?;

        Ok(ApiVersion(major, minor))
    }
}

impl Serialize for ApiVersion {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
            where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

struct ApiVersionVisitor;

impl<'de> Visitor<'de> for ApiVersionVisitor {
    type Value = ApiVersion;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in format X.Y")
    }

    fn visit_str<E>(self, value: &str) -> ::std::result::Result<ApiVersion, E>
            where E: DeserError {
        ApiVersion::from_str(value).map_err(DeserError::custom)
    }
}

impl<'de> Deserialize<'de> for ApiVersion {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<ApiVersion, D::Error>
            where D: Deserializer<'de> {
        deserializer.deserialize_str(ApiVersionVisitor)
    }
}


#[cfg(test)]
pub mod test {
    use std::str::FromStr;

    use serde_json;

    use super::ApiVersion;

    #[test]
    fn test_apiversion_format() {
        let ver = ApiVersion(2, 27);
        assert_eq!(&ver.to_string(), "2.27");
        assert_eq!(ApiVersion::from_str("2.27").unwrap(), ver);
    }

    #[test]
    fn test_apiversion_serde() {
        let ver = ApiVersion(2, 27);
        let ser = serde_json::to_string(&ver).unwrap();
        assert_eq!(&ser, "\"2.27\"");
        assert_eq!(serde_json::from_str::<ApiVersion>(&ser).unwrap(), ver);
    }
}
