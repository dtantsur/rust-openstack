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

use std::fmt::Debug;
use std::time::Duration;

use waiter::{Waiter, WaiterCurrentState};

use super::super::{Error, ErrorKind, Result};
use super::Refresh;


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

impl<T> WaiterCurrentState<T> for DeletionWaiter<T> {
    fn waiter_current_state(&self) -> &T {
        &self.inner
    }
}

impl<T: Refresh + Debug> Waiter<(), Error> for DeletionWaiter<T> {
    fn default_wait_timeout(&self) -> Option<Duration> {
        Some(self.wait_timeout)
    }

    fn default_delay(&self) -> Duration {
        self.delay
    }

    fn timeout_error(&self) -> Error {
        Error::new(ErrorKind::OperationTimedOut,
                   format!("Timeout waiting for resource {:?} to be deleted",
                           self.inner))
    }

    fn poll(&mut self) -> Result<Option<()>> {
        match self.inner.refresh() {
            Ok(..) => {
                trace!("Still waiting for resource {:?} to be deleted",
                       self.inner);
                Ok(None)
            },
            Err(ref e) if e.kind() == ErrorKind::ResourceNotFound => {
                debug!("Resource {:?} was deleted", self.inner);
                Ok(Some(()))
            },
            Err(e) => {
                debug!("Failed to delete resource {:?} - {}", self.inner, e);
                Err(e)
            }
        }
    }
}
