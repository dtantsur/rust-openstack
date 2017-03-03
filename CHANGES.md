ChangeLog
=========

next
----

* Prototype for ComputeApi: added list_servers().

0.0.5
-----

* auth::AuthToken is now a trait, with auth::base::SimpleAuthToken being
  the first implementation.
* ApiError::EndpointNotFound now contains the service type.

0.0.4
-----

* Turned AuthenticatedClient into Session.

0.0.3
-----

* Added session::AuthenticatedClient.
* Identity authentication: getting endpoint for service.

0.0.1
-----

* Identity authentication: receiving a token.
