# Managing migrations for Builder services

All builder migrations are run with [Diesel](http://diesel.rs). This document describes how to create and manage those migrations.

## Install the Diesel client

```shell
cargo install diesel_cli --version 2.0.0 --no-default-features --features postgres
```

**NOTE**: When this document was updated to add `--version 2.0.0` to the `cargo install` command above the current version of diesel.rs was v2.2.8 and builder itself was using v1.4.8. However, versions of the diesel_cli crate jump from 1.4.1 to 2.0.0-rc.0 and studying the diesel_cli crate history releases of diesel and diesel_cli are not 1:1 and it seems that a diesel_cli release, at least historically, is only cut as needed.  Ultimately diesel_cli v2.0.0 was chosen because `diesel generate pending` was erroring when executed when run on diesel 1.4.1.  Noticing that the last migration was dated 2022-08-09 version 2.0.0 was the diesel_clie release closest in time to migation that was also not a release candidate.  On v2.0.0 diesel migration seems to work as expected.

## Generating new migrations

Every time you need to make a change to the Builder schema you will be required to generate a new migration

For the service `builder-SERVICE` you will need to run:

* `cd components/builder-SERVICE/src`
* `diesel migration generate <your migration name>`

The migration name should describe what you are doing. Ex:

* create-posts
* add-user-select-v4
* remove-user-select-43

This will generate something like

```shell
Creating migrations/20160815133237_create_posts/up.sql
Creating migrations/20160815133237_create_posts/down.sql
```

You can then edit `up.sql` to create your migration steps.
You should ignore, but not delete, `down.sql` as we don't use it since we rely on transactions for our rollback logic.

## Testing your changes

You will need to compile your service and restart it to test your changes. You should see:

`Running Migration <your-migration-name>`
