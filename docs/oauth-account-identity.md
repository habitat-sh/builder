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

With `auto_provision_account` enabled, Builder stores `PROVISIONED_ACCOUNT_ID`
beside `HAB_AUTH_TOKEN` in the configured provisioning token directory. Keep
that directory across API restarts. It must be owned by the API service user
and accessible only to that user (mode `0700`); the two files must be regular,
service-owned files with no group or other access. Symlinks and hard links are
rejected. With `BLDR_TOKEN_DIR` unset, Builder uses a dedicated
`builder-api-provision` directory under the system temporary directory instead
of the shared temporary directory itself. The account ID lets provisioning reuse the
same row after an OAuth rename or token rotation. On upgrades without the ID
file, a still-valid `HAB_AUTH_TOKEN` can recover the account ID. If both files
are missing after the provisioned account has been renamed, restore the ID from
a verified account record; provisioning will not claim another OAuth account
by its username.
