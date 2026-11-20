# OAuth account identity

Builder identifies an OAuth account by its provider and the provider's stable
user ID. A username change updates `accounts.name` while preserving the account
ID, origins, memberships, and access tokens.

Existing accounts have no stored provider ID. On the first OAuth login, Builder
binds a same-name legacy row if its external ID is still null. An account that
already has an external ID is never claimed by name. Administrators can also
backfill verified provider IDs before rollout.

When a verified OAuth username is held by another account row, Builder moves
that row to a reserved name containing its account ID. Its account ID and
access remain intact, and the verified username points to the logging-in
account. Invitations' stored account names are updated with either rename.

A username reused before its legacy account is bound cannot be distinguished
from the original owner by username alone. Backfill those accounts from a
trusted provider ID source before enabling this login flow.

The account created by `auto_provision_account` is a service account for
startup API calls. It is never bound to an OAuth identity: while auto-provisioning
is enabled, an OAuth login whose username matches `provision.username` is rejected
with `409 Conflict`, so it cannot claim or rename that account. Provisioning
itself is unchanged, and upgrades need no manual steps.
