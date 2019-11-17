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

//! Stack management via Orchestration API.
use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use waiter::{Waiter, WaiterCurrentState};
use super::{Error, ErrorKind, Result};

mod protocol;
//mod api;

use super::common::{
    DeletionWaiter,
    Refresh,
};
use super::session::Session;

/// A request to create a stack.
#[derive(Debug)]
pub struct NewStack {
}

/// Structure representing a single stack.
#[derive(Clone, Debug)]
pub struct Stack {
    session: Rc<Session>,
    inner: protocol::Stack,
}

impl NewStack {
    /// Start creating a server.
    pub(crate) fn new(session: Rc<Session>, name: String) -> NewStack {
        NewServer {
         //   session,
        }
    }

    /// Request creation of the server.
    pub fn create(self) -> Result<StackCreationWaiter> {
        let request = protocol::StackCreate {
            name: self.name,
        };

        let stack_ref = api::create_stack(&self.session, request)?;
        Ok(StackCreationWaiter {
            stack: Stack::load(self.session, stack_ref.id)?,
        })
    }
}

/// Waiter for server to be created.
#[derive(Debug)]
pub struct StackCreationWaiter {
    stack: Stack,
}

/// Waiter for stack status to change.
#[derive(Debug)]
pub struct StackStatusWaiter<'stack> {
    stack: &'stack mut Stack,
    target: protocol::StackStatus,
}

impl Refresh for Stack {
    /// Refresh the stack.
    fn refresh(&mut self) -> Result<()> {
        //self.inner = api::get_stack_by_id(&self.session, &self.inner.id)?;
        Ok(())
    }
}


impl Stack {
    /// Create a new Stack object.
    pub(crate) fn new(session: Rc<Session>, inner: protocol::Stack) -> Result<Stack> {
        Ok(Stack {
            session,
            inner,
        })
    }

    /// Load a Stack object.
    pub(crate) fn load<Id: AsRef<str>>(session: Rc<Session>, id: Id) -> Result<Stack> {
        let inner = api::get_stack(&session, id)?;
        Stack::new(session, inner)
    }

    transparent_property! {
        #[doc = "Creation date and time."]
        created_at: DateTime<FixedOffset>
    }

    transparent_property! {
        #[doc = "Stack description."]
        description: ref Option<String>
    }

    transparent_property! {
        #[doc = "Stack unique ID."]
        id: ref String
    }

    transparent_property! {
        #[doc = "Stack name."]
        name: ref String
    }

    transparent_property! {
        #[doc = "Stack status."]
        status: protocol::StackStatus
    }

    /// Delete the stack.
    pub fn delete(self) -> Result<DeletionWaiter<Stack>> {
        api::delete_stack(&self.session, &self.inner.id)?;
        Ok(DeletionWaiter::new(
            self,
            Duration::new(120, 0),
            Duration::new(1, 0),
        ))
    }
}

impl<'server> Waiter<(), Error> for StackStatusWaiter<'server> {
    fn default_wait_timeout(&self) -> Option<Duration> {
        // TODO(dtantsur): vary depending on target?
        Some(Duration::new(600, 0))
    }

    fn default_delay(&self) -> Duration {
        Duration::new(1, 0)
    }

    fn timeout_error(&self) -> Error {
        Error::new(
            ErrorKind::OperationTimedOut,
            format!(
                "Timeout waiting for stack {} to reach state {}",
                self.stack.id(),
                self.target
            ),
        )
    }

    fn poll(&mut self) -> Result<Option<()>> {
        self.stack.refresh()?;
        if self.stack.status() == self.target {
            debug!("Stack {} reached state {}", self.stack.id(), self.target);
            Ok(Some(()))
        } else if self.stack.status() == protocol::StackStatus::Failed {
            debug!(
                "Failed to move stack {} to {} - status is ERROR",
                self.stack.id(),
                self.target
            );
            Err(Error::new(
                ErrorKind::OperationFailed,
                format!("Stack {} got into ERROR state", self.stack.id()),
            ))
        } else {
            trace!(
                "Still waiting for stack {} to get to state {}, current is {}",
                self.stack.id(),
                self.target,
                self.stack.status()
            );
            Ok(None)
        }
    }
}

impl<'server> WaiterCurrentState<Stack> for StackStatusWaiter<'server> {
    fn waiter_current_state(&self) -> &Stack {
        &self.stack
    }
}
