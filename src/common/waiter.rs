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

//! Waiters.

use std::time::{Duration, Instant};
use std::thread::sleep;

use super::super::{Error, ErrorKind, Result};
use super::{Refresh, ResourceId};

/// Trait representing a waiter for some asynchronous action to finish.
///
/// The type `T` is the final type of the action, while type `P` represents
/// an intermediate state.
pub trait Waiter<T, P=T> {
    /// Update the current state of the action.
    ///
    /// Returns `T` if the action is finished, `None` if it is not. All errors
    /// are propagated via the `Result`.
    ///
    /// This method should not be called again after it returned the final
    /// result.
    fn poll(&mut self) -> Result<Option<T>>;

    /// Default timeout for this action.
    ///
    /// This timeout is used in the `wait` method.
    /// If `None, wait forever by default.
    fn default_wait_timeout(&self) -> Option<Duration> { None }
    /// Default delay between two retries.
    ///
    /// The default is 0.1 seconds and should be changed by implementations.
    fn default_delay(&self) -> Duration {
        Duration::from_millis(100)
    }
    /// Error message to return on time out.
    fn timeout_error_message(&self) -> String {
        "Timeout while waiting for operation to finish".to_string()
    }

    /// Wait for the default amount of time.
    ///
    /// Returns `OperationTimedOut` if the timeout is reached.
    fn wait(self) -> Result<T> where Self: Sized {
        match self.default_wait_timeout() {
            Some(duration) => self.wait_for(duration),
            None => self.wait_forever()
        }
    }
    /// Wait for specified amount of time.
    ///
    /// Returns `OperationTimedOut` if the timeout is reached.
    fn wait_for(self, duration: Duration) -> Result<T> where Self: Sized{
        let delay = self.default_delay();
        self.wait_for_with_delay(duration, delay)
    }
    /// Wait for specified amount of time.
    ///
    /// Returns `OperationTimedOut` if the timeout is reached.
    fn wait_for_with_delay(mut self, duration: Duration, delay: Duration)
            -> Result<T> where Self: Sized {
        let start = Instant::now();
        while Instant::now().duration_since(start) <= duration {
            match self.poll()? {
                Some(result) => return Ok(result),
                None => ()  // continue
            };
            sleep(delay);
        };
        Err(Error::new(ErrorKind::OperationTimedOut,
                       self.timeout_error_message()))
    }
    /// Wait forever.
    fn wait_forever(self) -> Result<T> where Self: Sized {
        let delay = self.default_delay();
        self.wait_forever_with_delay(delay)
    }
    /// Wait forever with given delay between attempts.
    fn wait_forever_with_delay(mut self, delay: Duration)
            -> Result<T> where Self: Sized {
        loop {
            match self.poll()? {
                Some(result) => return Ok(result),
                None => ()  // continue
            };
            sleep(delay);
        }
    }
}

/// Wait for resource deletion.
#[derive(Debug)]
pub struct DeletionWaiter<T> {
    inner: T,
    wait_timeout: Duration,
    delay: Duration,
}

impl<T> DeletionWaiter<T> {
    #[allow(dead_code)]  // unused with --no-default-features
    pub(crate) fn new(inner: T, wait_timeout: Duration, delay: Duration)
            -> DeletionWaiter<T> {
        DeletionWaiter {
            inner: inner,
            wait_timeout: wait_timeout,
            delay: delay,
        }
    }
}

impl<T: ResourceId + Refresh> Waiter<()> for DeletionWaiter<T> {
    fn default_wait_timeout(&self) -> Option<Duration> {
        Some(self.wait_timeout)
    }

    fn default_delay(&self) -> Duration {
        self.delay
    }

    fn timeout_error_message(&self) -> String {
        format!("Timeout waiting for resource {} to be deleted",
                self.inner.resource_id())
    }

    fn poll(&mut self) -> Result<Option<()>> {
        match self.inner.refresh() {
            Ok(..) => {
                trace!("Still waiting for resource {} to be deleted",
                       self.inner.resource_id());
                Ok(None)
            },
            Err(ref e) if e.kind() == ErrorKind::ResourceNotFound => {
                debug!("Resource {} was deleted", self.inner.resource_id());
                Ok(Some(()))
            },
            Err(e) => {
                debug!("Failed to delete resource {} - {}",
                       self.inner.resource_id(), e);
                Err(e)
            }
        }
    }
}
