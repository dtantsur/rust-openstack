ChangeLog
=========

next
----

* Dropped ServiceApi struct in favor of using Session directly.
* Rework of the whole compute module.
* IdentityAuthMethod renamed to PasswordAuth.

0.0.8
-----

* Authentication method is using String class instead of string slices.
* Split Session::get_endpoint into get_default_endpoint and get_endpoint.
* Move region setting into Session from its endpoint calls.
* ServiceType now has a method to get root URL instead of relying on a suffix.
* Base support for getting API versions.

0.0.7
-----

* Major refactorings.
* Implement get(id) in the compute API.

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
