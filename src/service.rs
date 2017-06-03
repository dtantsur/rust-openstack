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

use std::marker::PhantomData;

use hyper::{Client, Url};
use hyper::client::{Body, IntoUrl, RequestBuilder as HyperRequestBuilder,
                    Response};
use hyper::header::{Header, HeaderFormat, Headers};
use hyper::method::Method;
use serde::{Deserialize, Serialize};
use serde_json;

use super::{ApiError, ApiResult, ApiVersion, ApiVersionRequest, Session};
use super::utils;


/// Request builder with error checking and JSON support.
///
/// Partly copies the interface of hyper::client::RequestBuilder.
#[allow(missing_debug_implementations)]
pub struct RequestBuilder<'a> {
    inner: HyperRequestBuilder<'a>
}

/// Type of query parameters.
#[derive(Clone, Debug)]
pub struct Query(pub Vec<(String, String)>);

/// Information about API endpoint.
#[derive(Clone, Debug)]
pub struct ServiceInfo {
    /// Root endpoint.
    pub root_url: Url,
    /// Current API version (if supported).
    pub current_version: Option<ApiVersion>,
    /// Minimum API version (if supported).
    pub minimum_version: Option<ApiVersion>
}

/// Trait representing a service type.
pub trait ServiceType {
    /// Service type to pass to the catalog.
    fn catalog_type() -> &'static str;

    /// Get basic service information.
    fn service_info(endpoint: Url, session: &Session)
        -> ApiResult<ServiceInfo>;
}

/// Trait representing a service with API version support.
pub trait ApiVersioning {
    /// Return headers to set for this API version.
    fn api_version_headers(version: ApiVersion) -> ApiResult<Headers>;
}

/// A service-specific wrapper around Session.
#[derive(Debug)]
pub struct ServiceWrapper<'session, Srv: ServiceType> {
    session: &'session Session,
    service_type: PhantomData<Srv>,
    endpoint_interface: Option<String>
}


impl Query {
    /// Empty query.
    pub fn new() -> Query {
        Query(Vec::new())
    }

    /// Add an item to the query.
    pub fn push<K, V>(&mut self, param: K, value: V)
            where K: Into<String>, V: ToString {
        self.0.push((param.into(), value.to_string()))
    }

    /// Add a strng item to the query.
    pub fn push_str<K, V>(&mut self, param: K, value: V)
            where K: Into<String>, V: Into<String> {
        self.0.push((param.into(), value.into()))
    }
}

impl<'a> RequestBuilder<'a> {
    /// Wrap a request builder.
    pub fn new<U>(client: &'a Client, method: Method, url: U, headers: Headers)
            -> RequestBuilder<'a> where U: IntoUrl {
        RequestBuilder {
            inner: client.request(method, url).headers(headers)
        }
    }

    /// Send this request.
    pub fn send(self) -> ApiResult<Response> {
        let resp = self.send_unchecked()?;
        if resp.status.is_success() {
            Ok(resp)
        } else {
            Err(ApiError::HttpError(resp.status, resp))
        }
    }

    /// Send this request without checking on status code.
    pub fn send_unchecked(self) -> ApiResult<Response> {
        self.inner.send().map_err(From::from).map(|resp| {
            trace!("Got {} from {} with {:?}",
                   resp.status, resp.url, resp.headers);
            resp
        })
    }

    /// Send this request and parse JSON response on success.
    pub fn fetch_json<T>(self) -> ApiResult<T>
            where for<'de> T: Deserialize<'de> {
        serde_json::from_reader(self.send()?).map_err(From::from)
    }

    /// Add body to the request.
    pub fn body<B: Into<Body<'a>>>(self, body: B) -> RequestBuilder<'a> {
        RequestBuilder {
            inner: self.inner.body(body)
        }
    }

    /// Add an individual header to the request.
    pub fn header<H: Header + HeaderFormat>(self, header: H)
            -> RequestBuilder<'a> {
        RequestBuilder {
            inner: self.inner.header(header)
        }
    }
}

impl<'session, Srv: ServiceType> ServiceWrapper<'session, Srv> {
    /// Create a new wrapper for the specific service.
    pub fn new(session: &'session Session) -> ServiceWrapper<'session, Srv> {
        ServiceWrapper {
            session: session,
            service_type: PhantomData,
            endpoint_interface: None
        }
    }

    /// Change the endpoint interface used for this wrapper.
    pub fn with_endpoint_interface(self, endpoint_interface: String)
            -> ServiceWrapper<'session, Srv> {
        ServiceWrapper {
            endpoint_interface: Some(endpoint_interface),
            .. self
        }
    }

    /// Construct and endpoint for the given service from the path.
    pub fn get_endpoint<P>(&self, path: P, query: Query) -> ApiResult<Url>
            where P: IntoIterator, P::Item: AsRef<str> {
        let ep = self.endpoint_interface.clone();
        let info = self.session.get_service_info::<Srv>(ep)?;
        let mut url = utils::url::extend(info.root_url, path);
        let _ = url.query_pairs_mut().extend_pairs(query.0);
        Ok(url)
    }

    /// Make an HTTP request to the given service.
    pub fn request<P>(&'session self, method: Method, path: P, query: Query)
            -> ApiResult<RequestBuilder<'session>>
            where P: IntoIterator, P::Item: AsRef<str> {
        let url = self.get_endpoint(path, query)?;
        let headers = self.session.service_headers::<Srv>();
        trace!("Sending HTTP {} request to {} with {:?}",
               method, url, headers);
        self.session.request(method, url, headers)
    }

    /// Make an HTTP request with JSON body and JSON response.
    pub fn json<P, Req, Res>(&self, method: Method, path: P, query: Query,
                             body: &Req) -> ApiResult<Res>
            where Req: Serialize, for<'de> Res: Deserialize<'de>,
            P: IntoIterator, P::Item: AsRef<str> {
        let str_body = serde_json::to_string(body)?;
        let request = self.request(method, path, query)?;
        request.body(&str_body).fetch_json()
    }

    /// Make a GET request returning a JSON.
    pub fn get_json<P, Res>(&self, path: P, query: Query) -> ApiResult<Res>
            where for<'de> Res: Deserialize<'de>, P: IntoIterator, P::Item: AsRef<str> {
        self.request(Method::Get, path, query)?.fetch_json()
    }

    /// Make a POST request sending and returning a JSON.
    pub fn post_json<P, Req, Res>(&self, path: P, query: Query, body: &Req)
            -> ApiResult<Res> where Req: Serialize, for <'de> Res: Deserialize<'de>,
            P: IntoIterator, P::Item: AsRef<str> {
        self.json(Method::Post, path, query, body)
    }

    /// Make a POST request sending and returning a JSON.
    pub fn put_json<P, Req, Res>(&self, path: P, query: Query, body: &Req)
            -> ApiResult<Res> where Req: Serialize, for<'de> Res: Deserialize<'de>,
            P: IntoIterator, P::Item: AsRef<str> {
        self.json(Method::Put, path, query, body)
    }

    /// Make a PATCH request sending and returning a JSON.
    pub fn patch_json<P, Req, Res>(&self, path: P, query: Query, body: &Req)
            -> ApiResult<Res> where Req: Serialize, for<'de> Res: Deserialize<'de>,
            P: IntoIterator, P::Item: AsRef<str> {
        self.json(Method::Patch, path, query, body)
    }

    /// Make a DELETE request.
    pub fn delete<P>(&self, path: P, query: Query) -> ApiResult<Response>
            where P: IntoIterator, P::Item: AsRef<str> {
        self.request(Method::Delete, path, query)?.send()
    }
}

impl<'session, Srv: ServiceType> Clone for ServiceWrapper<'session, Srv> {
    fn clone(&self) -> ServiceWrapper<'session, Srv> {
        ServiceWrapper {
            session: self.session,
            service_type: PhantomData,
            endpoint_interface: self.endpoint_interface.clone()
        }
    }
}

impl ServiceInfo {
    /// Pick an API version.
    pub fn pick_api_version(&self, request: ApiVersionRequest)
            -> Option<ApiVersion> {
        match request {
            ApiVersionRequest::Minimum =>
                self.minimum_version,
            ApiVersionRequest::Latest =>
                self.current_version,
            ApiVersionRequest::Exact(req) => {
                self.current_version.and_then(|max| {
                    match self.minimum_version {
                        Some(min) if req >= min && req <= max => Some(req),
                        None if req == max => Some(req),
                        _ => None
                    }
                })
            },
            ApiVersionRequest::Choice(vec) => {
                if vec.is_empty() {
                    return None;
                }

                self.current_version.and_then(|max| {
                    match self.minimum_version {
                        Some(min) => vec.into_iter().filter(|x| {
                            *x >= min && *x <= max
                        }).max(),
                        None =>vec.into_iter().find(|x| *x == max)
                    }
                })
            }
        }
    }
}


#[cfg(test)]
pub mod test {
    use hyper::Url;

    use super::super::{ApiVersion, ApiVersionRequest};
    use super::ServiceInfo;

    fn service_info(min: Option<u16>, max: Option<u16>) -> ServiceInfo {
        ServiceInfo {
            root_url: Url::parse("http://127.0.0.1").unwrap(),
            minimum_version: min.map(|x| ApiVersion(2, x)),
            current_version: max.map(|x| ApiVersion(2, x)),
        }
    }

    #[test]
    fn test_pick_version_exact() {
        let info = service_info(Some(1), Some(24));
        let version = ApiVersion(2, 22);
        let result = info.pick_api_version(ApiVersionRequest::Exact(version))
            .unwrap();
        assert_eq!(result, version);
    }

    #[test]
    fn test_pick_version_exact_mismatch() {
        let info = service_info(Some(1), Some(24));
        let version = ApiVersion(2, 25);
        let res1 = info.pick_api_version(ApiVersionRequest::Exact(version));
        assert!(res1.is_none());
        let version2 = ApiVersion(1, 11);
        let res2 = info.pick_api_version(ApiVersionRequest::Exact(version2));
        assert!(res2.is_none());
    }

    #[test]
    fn test_pick_version_exact_current_only() {
        let info = service_info(None, Some(24));
        let version = ApiVersion(2, 24);
        let result = info.pick_api_version(ApiVersionRequest::Exact(version))
            .unwrap();
        assert_eq!(result, version);
    }

    #[test]
    fn test_pick_version_exact_current_only_mismatch() {
        let info = service_info(None, Some(24));
        let version = ApiVersion(2, 22);
        let result = info.pick_api_version(ApiVersionRequest::Exact(version));
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_minimum() {
        let info = service_info(Some(1), Some(24));
        let result = info.pick_api_version(ApiVersionRequest::Minimum)
            .unwrap();
        assert_eq!(result, ApiVersion(2, 1));
    }

    #[test]
    fn test_pick_version_minimum_unknown() {
        let info = service_info(None, Some(24));
        let result = info.pick_api_version(ApiVersionRequest::Minimum);
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_latest() {
        let info = service_info(Some(1), Some(24));
        let result = info.pick_api_version(ApiVersionRequest::Latest)
            .unwrap();
        assert_eq!(result, ApiVersion(2, 24));
    }

    #[test]
    fn test_pick_version_latest_unknown() {
        let info = service_info(Some(1), None);
        let result = info.pick_api_version(ApiVersionRequest::Latest);
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_choice() {
        let info = service_info(Some(1), Some(24));
        let choice = vec![ApiVersion(2, 0), ApiVersion(2, 2),
                          ApiVersion(2, 22), ApiVersion(2, 25)];
        let result = info.pick_api_version(ApiVersionRequest::Choice(choice))
            .unwrap();
        assert_eq!(result, ApiVersion(2, 22));
    }

    #[test]
    fn test_pick_version_choice_mismatch() {
        let info = service_info(Some(1), Some(24));
        let choice = vec![ApiVersion(2, 0), ApiVersion(2, 25)];
        let result = info.pick_api_version(ApiVersionRequest::Choice(choice));
        assert!(result.is_none());
    }

    #[test]
    fn test_pick_version_choice_current_only() {
        let info = service_info(None, Some(24));
        let choice = vec![ApiVersion(2, 0), ApiVersion(2, 2),
                          ApiVersion(2, 24), ApiVersion(2, 25)];
        let result = info.pick_api_version(ApiVersionRequest::Choice(choice))
            .unwrap();
        assert_eq!(result, ApiVersion(2, 24));
    }

    #[test]
    fn test_pick_version_choice_current_only_mismatch() {
        let info = service_info(None, Some(24));
        let choice = vec![ApiVersion(2, 0), ApiVersion(2, 2),
                          ApiVersion(2, 22), ApiVersion(2, 25)];
        let result = info.pick_api_version(ApiVersionRequest::Choice(choice));
        assert!(result.is_none());
    }
}
