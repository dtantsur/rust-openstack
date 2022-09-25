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

//! Synchronous wrapper for a session.
//!
//! This module is only available when the `sync` feature is enabled.

use std::cell::RefCell;
use std::io;
use std::pin::Pin;
use std::result;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::executor::{self, BlockingStream};
use futures::stream::Stream;
use futures::Future;
use osauth::request;
use osauth::services::ServiceType;
use osauth::{ApiVersion, AuthType, EndpointFilters, Error, InterfaceType, Session};
use pin_project::pin_project;
use reqwest::{Body, RequestBuilder, Response};
use reqwest::{Method, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

/// A result of an OpenStack operation.
pub type Result<T> = result::Result<T, Error>;

/// An item in a `SyncStream`.
pub type SyncStreamItem = result::Result<Bytes, ::reqwest::Error>;

/// A reader into an asynchronous stream.
#[derive(Debug)]
pub struct SyncStream<S, E = ::reqwest::Error>
where
    S: Stream<Item = result::Result<Bytes, E>> + Unpin,
{
    inner: BlockingStream<S>,
    current: io::Cursor<Bytes>,
}

/// A synchronous body that can be used with asynchronous code.
#[pin_project]
#[derive(Debug, Clone, Default)]
pub struct SyncBody<R> {
    reader: R,
}

/// A synchronous wrapper for an asynchronous session.
#[derive(Debug)]
pub struct SyncSession {
    inner: Session,
    runtime: RefCell<Runtime>,
}

impl From<SyncSession> for Session {
    fn from(value: SyncSession) -> Session {
        value.inner
    }
}

impl From<Session> for SyncSession {
    fn from(value: Session) -> SyncSession {
        SyncSession::new(value)
    }
}

impl Clone for SyncSession {
    fn clone(&self) -> SyncSession {
        SyncSession::new(self.inner.clone())
    }
}

impl SyncSession {
    /// Create a new synchronous wrapper.
    ///
    /// Panics if unable to create a single-threaded runtime.
    pub fn new(session: Session) -> SyncSession {
        SyncSession {
            inner: session,
            runtime: RefCell::new(
                RuntimeBuilder::new()
                    .basic_scheduler()
                    .enable_io()
                    .build()
                    .expect("Could not create a runtime"),
            ),
        }
    }

    /// Create a `SyncSession` from a `clouds.yaml` configuration file.
    ///
    /// See [Session::from_config](../struct.Session.html#method.from_config) for details.
    #[inline]
    pub fn from_config<S: AsRef<str>>(cloud_name: S) -> Result<SyncSession> {
        Ok(Self::new(Session::from_config(cloud_name)?))
    }

    /// Create a `SyncSession` from environment variables.
    ///
    /// See [Session::from_env](../struct.Session.html#method.from_env) for details.
    #[inline]
    pub fn from_env() -> Result<SyncSession> {
        Ok(Self::new(Session::from_env()?))
    }

    /// Get a reference to the authentication type in use.
    #[inline]
    pub fn auth_type(&self) -> &dyn AuthType {
        self.inner.auth_type()
    }

    /// Endpoint interface in use (if any).
    #[inline]
    pub fn endpoint_filters(&self) -> &EndpointFilters {
        self.inner.endpoint_filters()
    }

    /// Modify endpoint filters.
    ///
    /// This call clears the cached service information for this `Session`.
    /// It does not, however, affect clones of this `Session`.
    #[inline]
    pub fn endpoint_filters_mut(&mut self) -> &mut EndpointFilters {
        self.inner.endpoint_filters_mut()
    }

    /// Refresh the session.
    #[inline]
    pub fn refresh(&mut self) -> Result<()> {
        let fut = self.inner.refresh();
        self.runtime.borrow_mut().block_on(fut)
    }

    /// Reference to the asynchronous session used.
    #[inline]
    pub fn session(&self) -> &Session {
        &self.inner
    }

    /// Set a new authentication for this `Session`.
    ///
    /// This call clears the cached service information for this `Session`.
    /// It does not, however, affect clones of this `Session`.
    #[inline]
    pub fn set_auth_type<Auth: AuthType + 'static>(&mut self, auth_type: Auth) {
        self.inner.set_auth_type(auth_type);
    }

    /// A convenience call to set an endpoint interface.
    ///
    /// This call clears the cached service information for this `Session`.
    /// It does not, however, affect clones of this `Session`.
    pub fn set_endpoint_interface(&mut self, endpoint_interface: InterfaceType) {
        self.inner.set_endpoint_interface(endpoint_interface);
    }

    /// Convert this session into one using the given authentication.
    #[inline]
    pub fn with_auth_type<Auth: AuthType + 'static>(mut self, auth_method: Auth) -> SyncSession {
        self.set_auth_type(auth_method);
        self
    }

    /// Convert this session into one using the given endpoint filters.
    #[inline]
    pub fn with_endpoint_filters(mut self, endpoint_filters: EndpointFilters) -> SyncSession {
        *self.endpoint_filters_mut() = endpoint_filters;
        self
    }

    /// Convert this session into one using the given endpoint filters.
    #[inline]
    pub fn with_endpoint_interface(mut self, endpoint_interface: InterfaceType) -> SyncSession {
        self.set_endpoint_interface(endpoint_interface);
        self
    }

    /// Get minimum/maximum API (micro)version information.
    ///
    /// Returns `None` if the range cannot be determined, which usually means
    /// that microversioning is not supported.
    ///
    /// ```rust,no_run
    /// let session = openstack::session::Session::from_env()
    ///     .expect("Failed to create an identity provider from the environment");
    /// let maybe_versions = session
    ///     .get_api_versions(osauth::services::COMPUTE)
    ///     .expect("Cannot determine supported API versions");
    /// if let Some((min, max)) = maybe_versions {
    ///     println!("The compute service supports versions {} to {}", min, max);
    /// } else {
    ///     println!("The compute service does not support microversioning");
    /// }
    /// ```
    #[inline]
    pub fn get_api_versions<Srv>(&self, service: Srv) -> Result<Option<(ApiVersion, ApiVersion)>>
    where
        Srv: ServiceType + Send,
    {
        self.block_on(self.inner.get_api_versions(service))
    }

    /// Construct and endpoint for the given service from the path.
    ///
    /// You won't need to use this call most of the time, since all request calls can fetch the
    /// endpoint automatically.
    #[inline]
    pub fn get_endpoint<Srv, I>(&self, service: Srv, path: I) -> Result<Url>
    where
        Srv: ServiceType + Send,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
    {
        self.block_on(self.inner.get_endpoint(service, path))
    }

    /// Get the currently used major version from the given service.
    ///
    /// Can return `None` if the service does not support API version discovery at all.
    #[inline]
    pub fn get_major_version<Srv>(&self, service: Srv) -> Result<Option<ApiVersion>>
    where
        Srv: ServiceType + Send,
    {
        self.block_on(self.inner.get_major_version(service))
    }

    /// Pick the highest API version supported by the service.
    ///
    /// Returns `None` if none of the requested versions are available.
    ///
    /// ```rust,no_run
    /// let session = openstack::session::Session::from_env()
    ///     .expect("Failed to create an identity provider from the environment");
    /// let candidates = vec![osauth::ApiVersion(1, 2), osauth::ApiVersion(1, 42)];
    /// let maybe_version = session
    ///     .pick_api_version(osauth::services::COMPUTE, candidates)
    ///     .expect("Cannot negotiate an API version");
    /// if let Some(version) = maybe_version {
    ///     println!("Using version {}", version);
    /// } else {
    ///     println!("Using the base version");
    /// }
    /// ```
    #[inline]
    pub fn pick_api_version<Srv, I>(&self, service: Srv, versions: I) -> Result<Option<ApiVersion>>
    where
        Srv: ServiceType + Send,
        I: IntoIterator<Item = ApiVersion>,
        I::IntoIter: Send,
    {
        self.block_on(self.inner.pick_api_version(service, versions))
    }

    /// Check if the service supports the API version.
    #[inline]
    pub fn supports_api_version<Srv: ServiceType + Send>(
        &self,
        service: Srv,
        version: ApiVersion,
    ) -> Result<bool> {
        self.block_on(self.inner.supports_api_version(service, version))
    }

    /// Make an HTTP request to the given service.
    ///
    /// The `service` argument is an object implementing the
    /// [ServiceType](../services/trait.ServiceType.html) trait. Some known service types are
    /// available in the [services](../services/index.html) module.
    ///
    /// The `path` argument is a URL path without the service endpoint (e.g. `/servers/1234`).
    ///
    /// If `api_version` is set, it is send with the request to enable a higher API version.
    /// Otherwise the base API version is used. You can use
    /// [pick_api_version](#method.pick_api_version) to choose an API version to use.
    ///
    /// The result is a `RequestBuilder` that can be customized further. Error checking and response
    /// parsing can be done using e.g. [send_checked](#method.send_checked) or
    /// [fetch_json](#method.fetch_json).
    ///
    /// ```rust,no_run
    /// use reqwest::Method;
    ///
    /// let session = openstack::session::Session::from_env()
    ///     .expect("Failed to create an identity provider from the environment");
    /// session
    ///     .request(osauth::services::COMPUTE, Method::HEAD, &["servers", "1234"], None)
    ///     .and_then(|builder| session.send_checked(builder))
    ///     .map(|response| {
    ///         println!("Response: {:?}", response);
    ///     });
    /// ```
    ///
    /// This is the most generic call to make a request. You may prefer to use more specific `get`,
    /// `post`, `put` or `delete` calls instead.
    pub fn request<Srv, I>(
        &self,
        service: Srv,
        method: Method,
        path: I,
        api_version: Option<ApiVersion>,
    ) -> Result<RequestBuilder>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
    {
        self.block_on(self.inner.request(service, method, path, api_version))
    }

    /// Issue a GET request.
    ///
    /// See [request](#method.request) for an explanation of the parameters.
    #[inline]
    pub fn get<Srv, I>(
        &self,
        service: Srv,
        path: I,
        api_version: Option<ApiVersion>,
    ) -> Result<Response>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
    {
        self.send_checked(self.request(service, Method::GET, path, api_version)?)
    }

    /// Fetch a JSON using the GET request.
    ///
    /// ```rust,no_run
    /// use osproto::common::IdAndName;
    /// use serde::Deserialize;
    ///
    /// #[derive(Debug, Deserialize)]
    /// pub struct ServersRoot {
    ///     pub servers: Vec<IdAndName>,
    /// }
    ///
    /// let session = openstack::session::Session::from_env()
    ///     .expect("Failed to create an identity provider from the environment");
    ///
    /// session
    ///     .get_json(osauth::services::COMPUTE, &["servers"], None)
    ///     .map(|servers: ServersRoot| {
    ///         for srv in servers.servers {
    ///             println!("ID = {}, Name = {}", srv.id, srv.name);
    ///         }
    ///     });
    /// ```
    ///
    /// See [request](#method.request) for an explanation of the parameters.
    #[inline]
    pub fn get_json<Srv, I, T>(
        &self,
        service: Srv,
        path: I,
        api_version: Option<ApiVersion>,
    ) -> Result<T>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
        T: DeserializeOwned + Send,
    {
        self.fetch_json(self.request(service, Method::GET, path, api_version)?)
    }

    /// Fetch a JSON using the GET request with a query.
    ///
    /// See `reqwest` crate documentation for how to define a query.
    /// See [request](#method.request) for an explanation of the parameters.
    #[inline]
    pub fn get_json_query<Srv, I, Q, T>(
        &self,
        service: Srv,
        path: I,
        query: Q,
        api_version: Option<ApiVersion>,
    ) -> Result<T>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
        Q: Serialize + Send,
        T: DeserializeOwned + Send,
    {
        self.fetch_json(
            self.request(service, Method::GET, path, api_version)?
                .query(&query),
        )
    }

    /// Issue a GET request with a query
    ///
    /// See `reqwest` crate documentation for how to define a query.
    /// See [request](#method.request) for an explanation of the parameters.
    #[inline]
    pub fn get_query<Srv, I, Q>(
        &self,
        service: Srv,
        path: I,
        query: Q,
        api_version: Option<ApiVersion>,
    ) -> Result<Response>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
        Q: Serialize + Send,
    {
        self.send_checked(
            self.request(service, Method::GET, path, api_version)?
                .query(&query),
        )
    }

    /// Download a body from a response.
    ///
    /// ```rust,no_run
    /// use std::io::Read;
    ///
    /// let session = openstack::session::Session::from_env()
    ///     .expect("Failed to create an identity provider from the environment");
    ///
    /// session
    ///     .get(osauth::services::OBJECT_STORAGE, &["test-container", "test-object"], None)
    ///     .map(|response| {
    ///         let mut buffer = Vec::new();
    ///         session
    ///             .download(response)
    ///             .read_to_end(&mut buffer)
    ///             .map(|_| {
    ///                 println!("Data: {:?}", buffer);
    ///             })
    ///             // Do not do this in production!
    ///             .expect("Could not read the remote file")
    ///     })
    ///     .expect("Could not open the remote file");
    ///
    /// ```
    #[inline]
    pub fn download(&self, response: Response) -> SyncStream<impl Stream<Item = SyncStreamItem>> {
        SyncStream::new(response.bytes_stream())
    }

    /// POST a JSON object.
    ///
    /// The `body` argument is anything that can be serialized into JSON.
    ///
    /// See [request](#method.request) for an explanation of the other parameters.
    #[inline]
    pub fn post<Srv, I, T>(
        &self,
        service: Srv,
        path: I,
        body: T,
        api_version: Option<ApiVersion>,
    ) -> Result<Response>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
        T: Serialize + Send,
    {
        self.send_checked(
            self.request(service, Method::POST, path, api_version)?
                .json(&body),
        )
    }

    /// POST a JSON object and receive a JSON back.
    ///
    /// The `body` argument is anything that can be serialized into JSON.
    ///
    /// See [request](#method.request) for an explanation of the other parameters.
    #[inline]
    pub fn post_json<Srv, I, T, R>(
        &self,
        service: Srv,
        path: I,
        body: T,
        api_version: Option<ApiVersion>,
    ) -> Result<R>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
        T: Serialize + Send,
        R: DeserializeOwned + Send,
    {
        self.fetch_json(
            self.request(service, Method::POST, path, api_version)?
                .json(&body),
        )
    }

    /// PUT a JSON object.
    ///
    /// The `body` argument is anything that can be serialized into JSON.
    ///
    /// See [request](#method.request) for an explanation of the other parameters.
    #[inline]
    pub fn put<Srv, I, T>(
        &self,
        service: Srv,
        path: I,
        body: T,
        api_version: Option<ApiVersion>,
    ) -> Result<Response>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
        T: Serialize + Send,
    {
        self.send_checked(
            self.request(service, Method::PUT, path, api_version)?
                .json(&body),
        )
    }

    /// Issue an empty PUT request.
    ///
    /// See [request](#method.request) for an explanation of the parameters.
    #[inline]
    pub fn put_empty<Srv, I>(
        &self,
        service: Srv,
        path: I,
        api_version: Option<ApiVersion>,
    ) -> Result<Response>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
    {
        self.send_checked(self.request(service, Method::PUT, path, api_version)?)
    }

    /// PUT a JSON object and receive a JSON back.
    ///
    /// The `body` argument is anything that can be serialized into JSON.
    ///
    /// See [request](#method.request) for an explanation of the other parameters.
    #[inline]
    pub fn put_json<Srv, I, T, R>(
        &self,
        service: Srv,
        path: I,
        body: T,
        api_version: Option<ApiVersion>,
    ) -> Result<R>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
        T: Serialize + Send,
        R: DeserializeOwned + Send,
    {
        self.fetch_json(
            self.request(service, Method::PUT, path, api_version)?
                .json(&body),
        )
    }

    /// Issue a DELETE request.
    ///
    /// See [request](#method.request) for an explanation of the parameters.
    #[inline]
    pub fn delete<Srv, I>(
        &self,
        service: Srv,
        path: I,
        api_version: Option<ApiVersion>,
    ) -> Result<Response>
    where
        Srv: ServiceType + Send + Clone,
        I: IntoIterator,
        I::Item: AsRef<str>,
        I::IntoIter: Send,
    {
        self.send_checked(self.request(service, Method::DELETE, path, api_version)?)
    }

    /// Send the response and convert the response to a JSON.
    #[inline]
    pub fn fetch_json<T>(&self, builder: RequestBuilder) -> Result<T>
    where
        T: DeserializeOwned + Send,
    {
        self.block_on(async { request::to_json(builder.send().await?).await })
    }

    /// Check the response and convert errors into OpenStack ones.
    #[inline]
    pub fn send_checked(&self, builder: RequestBuilder) -> Result<Response> {
        self.block_on(async { request::check(builder.send().await?).await })
    }

    #[inline]
    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future,
    {
        self.runtime.borrow_mut().block_on(f)
    }
}

impl<S, E> SyncStream<S, E>
where
    S: Stream<Item = result::Result<Bytes, E>> + Unpin,
{
    fn new(inner: S) -> SyncStream<S, E> {
        SyncStream {
            inner: executor::block_on_stream(inner),
            current: io::Cursor::default(),
        }
    }
}

impl<S, E> io::Read for SyncStream<S, E>
where
    S: Stream<Item = result::Result<Bytes, E>> + Unpin,
    E: Into<Box<dyn ::std::error::Error + Send + Sync + 'static>>,
{
    /// Read a chunk for the asynchronous stream.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let existing = self.current.read(buf)?;
            if existing > 0 {
                // Read something from the current cursor, can quit for now.
                return Ok(existing);
            }

            if let Some(next) = self.inner.next() {
                self.current =
                    io::Cursor::new(next.map_err(|err| io::Error::new(io::ErrorKind::Other, err))?);
            } else {
                return Ok(0);
            }
        }
    }
}

impl<R> SyncBody<R> {
    /// Create a new body from a reader.
    #[inline]
    pub fn new(body: R) -> SyncBody<R> {
        SyncBody { reader: body }
    }
}

impl<R> Stream for SyncBody<R>
where
    R: io::Read,
{
    type Item = ::std::result::Result<Bytes, io::Error>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut buffer = vec![0; 16384];
        let reader = self.project().reader;
        // FIXME(dtantsur): this blocks, we need to move it to another thread?
        let size = reader.read(&mut buffer)?;
        Poll::Ready(if size > 0 {
            buffer.truncate(size);
            Some(Ok(buffer.into()))
        } else {
            None
        })
    }
}

impl<R> From<SyncBody<R>> for Body
where
    R: io::Read + Send + Sync + 'static,
{
    fn from(value: SyncBody<R>) -> Body {
        Body::wrap_stream(value)
    }
}

#[cfg(test)]
mod test {
    use std::io::{Cursor, Read};

    use bytes::Bytes;
    use futures::stream;
    use osauth::Error;
    use reqwest::{Body, Error as HttpError};

    use super::{SyncBody, SyncStream};

    #[test]
    fn test_stream_empty() {
        let inner = stream::empty::<Result<Bytes, HttpError>>();
        let mut st = SyncStream::new(inner);
        let mut buffer = Vec::new();
        assert_eq!(0, st.read_to_end(&mut buffer).unwrap());
    }

    #[test]
    fn test_stream_all() {
        let data: Vec<Result<Bytes, Error>> = vec![
            Ok(Bytes::from(vec![1u8, 2, 3])),
            Ok(Bytes::from(vec![4u8])),
            Ok(Bytes::from(vec![5u8, 6])),
        ];
        let mut st = SyncStream::new(stream::iter(data.into_iter()));
        let mut buffer = Vec::new();
        assert_eq!(6, st.read_to_end(&mut buffer).unwrap());
        assert_eq!(vec![1, 2, 3, 4, 5, 6], buffer);
    }

    #[test]
    fn test_stream_parts() {
        let data: Vec<Result<Bytes, Error>> = vec![
            Ok(Bytes::from(vec![1u8, 2, 3])),
            Ok(Bytes::from(vec![4u8])),
            Ok(Bytes::from(vec![5u8, 6, 7, 8])),
        ];
        let mut st = SyncStream::new(stream::iter(data.into_iter()));
        let mut buffer = [0; 3];
        assert_eq!(3, st.read(&mut buffer).unwrap());
        assert_eq!([1, 2, 3], buffer);
        assert_eq!(1, st.read(&mut buffer).unwrap());
        assert_eq!([4, 2, 3], buffer);
        assert_eq!(3, st.read(&mut buffer).unwrap());
        assert_eq!([5, 6, 7], buffer);
        assert_eq!(1, st.read(&mut buffer).unwrap());
        assert_eq!([8, 6, 7], buffer);
        assert_eq!(0, st.read(&mut buffer).unwrap());
    }

    #[test]
    fn test_body() {
        let data = vec![42; 16_777_000]; // a bit short of 16 MiB
        let body = SyncBody::new(Cursor::new(data));
        let mut st = SyncStream::new(body);
        let mut buffer = Vec::new();
        assert_eq!(16_777_000, st.read_to_end(&mut buffer).unwrap());
    }

    #[test]
    fn test_body_to_chunk() {
        let data = vec![42; 16_777_000]; // a bit short of 16 MiB
        let body = SyncBody::new(Cursor::new(data));
        let _ = Body::from(body);
    }
}
