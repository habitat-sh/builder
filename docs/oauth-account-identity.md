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

## Before upgrading an auto-provisioned installation

Stop `builder-api` before changing provisioning files. The new startup checks
require the provisioning directory to be owned by the API service user with
mode `0700` (or stricter). `HAB_AUTH_TOKEN` and, if present,
`PROVISIONED_ACCOUNT_ID` must be regular files owned by that same user, with
mode `0600` (or stricter) and exactly one link. Symlinks are rejected. Under
the former default, `HAB_AUTH_TOKEN` was written directly to the shared
temporary directory, often as mode `0644`; do **not** change the permissions
of `/tmp` or trust an account ID found there.

For the former default, or whenever the existing directory or files cannot be
verified, use a new, previously absent directory under a trusted parent:

1. Determine the provisioned account ID from a trusted database record or
   backup. Confirm that the account owns the expected origins. Do not obtain
   this ID solely from the old `PROVISIONED_ACCOUNT_ID` file.
2. As an administrator, create the new directory for the API service user:
   `install -d -o <service-user> -g <service-group> -m 0700 <new-directory>`.
   Use a persistent path, such as `/var/lib/builder-api/provision`, whose
   parent cannot be modified by other local users.
3. As the service user, write the verified decimal ID to
   `<new-directory>/PROVISIONED_ACCOUNT_ID` with `umask 077`, and verify that
   the file is owned by the service user and has mode `0600`. Leave
   `HAB_AUTH_TOKEN` absent. At startup, Builder selects the verified account
   by ID, retrieves its token from the database (or creates one), and writes
   both provisioning files with restricted permissions.
4. Set `provision.token_path` or `BLDR_TOKEN_DIR` to the new directory and
   restart `builder-api`. Keep that setting and directory across restarts.

For an existing dedicated directory, an in-place permission change is safe
only after verifying that the directory and each existing provisioning file
are service-owned, the files are regular with one link, no path component is
an untrusted symlink, and any stored account ID matches the trusted account
record. With the API stopped, set the directory to mode `0700` and each
existing `HAB_AUTH_TOKEN` and `PROVISIONED_ACCOUNT_ID` file to mode `0600`,
then restart. If any check fails, follow the new-directory procedure instead.
An old token that was readable by other users should be rotated after the
upgrade.
