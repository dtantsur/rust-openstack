ChangeLog
=========

next
----

0.0.6
-----

* Started compute API: added compute::servers().
* Added ServerManager::list() for listing without filtering.
* Added compute::ServerFilters for future filtering.
* Generic ServiceApi implementation.
* Introduce ApiVersion structure.

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
