use super::db_id_format;
use chrono::NaiveDateTime;
use diesel::{self,
             pg::PgConnection,
             result::QueryResult,
             sql_query,
             Connection,
             ExpressionMethods,
             OptionalExtension,
             QueryDsl,
             RunQueryDsl};

use crate::{bldr_core::metrics::CounterMetric,
            metrics::Counter,
            schema::{account::{account_tokens,
                               accounts},
                     invitation::origin_invitations}};

#[derive(Debug, Identifiable, Serialize, Queryable)]
pub struct Account {
    #[serde(with = "db_id_format")]
    pub id:         i64,
    pub email:      String,
    pub name:       String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    #[serde(skip_serializing)]
    pub provider:   Option<String>,
    #[serde(skip_serializing)]
    pub extern_id:  Option<String>,
}

#[derive(Identifiable, Debug, Serialize, Queryable)]
#[diesel(table_name = account_tokens)]
pub struct AccountToken {
    #[serde(with = "db_id_format")]
    pub id:         i64,
    #[serde(with = "db_id_format")]
    pub account_id: i64,
    pub token:      String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = accounts)]
pub struct NewAccount<'a> {
    pub email: &'a str,
    pub name:  &'a str,
}

impl Account {
    pub fn get(name: &str, conn: &mut PgConnection) -> QueryResult<Account> {
        Counter::DBCall.increment();
        accounts::table.filter(accounts::name.eq(name))
                       .get_result(conn)
    }

    pub fn get_by_id(id: i64, conn: &mut PgConnection) -> QueryResult<Account> {
        Counter::DBCall.increment();
        accounts::table.find(id).get_result(conn)
    }

    pub fn create(account: &NewAccount, conn: &mut PgConnection) -> QueryResult<Account> {
        Counter::DBCall.increment();
        diesel::insert_into(accounts::table).values(account)
                                            .get_result(conn)
    }

    pub fn find_or_create(account: &NewAccount, conn: &mut PgConnection) -> QueryResult<Account> {
        Counter::DBCall.increment();
        match diesel::insert_into(accounts::table).values(account)
                                                  .on_conflict(accounts::name)
                                                  .do_nothing()
                                                  .get_result(conn)
        {
            Ok(account) => Ok(account),
            Err(_) => {
                // Provisioning must never acquire an OAuth-bound account by name.
                accounts::table.filter(accounts::name.eq(account.name))
                               .filter(accounts::extern_id.is_null())
                               .get_result(conn)
            }
        }
    }

    /// Resolve an OAuth login by provider identity, binding a same-name
    /// legacy row on its first login. A row with an external ID is never
    /// claimed by name.
    pub fn find_or_create_oauth(account: &NewAccount,
                                provider: &str,
                                extern_id: &str,
                                conn: &mut PgConnection)
                                -> QueryResult<Account> {
        if provider.is_empty() || extern_id.is_empty() || account.name.is_empty() {
            return Err(diesel::result::Error::NotFound);
        }
        conn.transaction(|conn| {
                // Serialise name changes, legacy claims and new account creation.
                sql_query("LOCK TABLE accounts IN SHARE ROW EXCLUSIVE MODE").execute(conn)?;

                let identity = accounts::table.filter(accounts::provider.eq(provider))
                                              .filter(accounts::extern_id.eq(extern_id))
                                              .first::<Account>(conn)
                                              .optional()?;

                let occupant = accounts::table.filter(accounts::name.eq(account.name))
                                              .first::<Account>(conn)
                                              .optional()?;

                let account_id = if let Some(ref identity) = identity {
                    identity.id
                } else if let Some(ref legacy) = occupant {
                    if legacy.extern_id.is_none() && legacy.provider.is_none() {
                        legacy.id
                    } else {
                        0
                    }
                } else {
                    0
                };

                // The incoming provider owns its asserted username. Move any
                // other occupant aside without changing its id or access.
                if let Some(occupant) = occupant {
                    if occupant.id != account_id {
                        let mut reserved = format!("__renamed_account_{}", occupant.id);
                        let mut suffix = 0;
                        while accounts::table.filter(accounts::name.eq(&reserved))
                                             .first::<Account>(conn)
                                             .optional()?
                                             .is_some()
                        {
                            suffix += 1;
                            reserved = format!("__renamed_account_{}_{}", occupant.id, suffix);
                        }
                        diesel::update(accounts::table.find(occupant.id))
                        .set(accounts::name.eq(&reserved))
                        .execute(conn)?;
                        diesel::update(origin_invitations::table
                                   .filter(origin_invitations::account_id.eq(occupant.id)))
                        .set(origin_invitations::account_name.eq(&reserved))
                        .execute(conn)?;
                    }
                }

                let resolved: Account = if account_id != 0 {
                    diesel::update(accounts::table.find(account_id))
                    .set((accounts::name.eq(account.name),
                          accounts::provider.eq(Some(provider)),
                          accounts::extern_id.eq(Some(extern_id))))
                    .get_result(conn)
                } else {
                    diesel::insert_into(accounts::table)
                    .values((accounts::name.eq(account.name),
                             accounts::email.eq(account.email),
                             accounts::provider.eq(Some(provider)),
                             accounts::extern_id.eq(Some(extern_id))))
                    .get_result(conn)
                }?;
                diesel::update(origin_invitations::table
                           .filter(origin_invitations::account_id.eq(resolved.id)))
                .set(origin_invitations::account_name.eq(&resolved.name))
                .execute(conn)?;
                Ok(resolved)
            })
    }

    pub fn update(id: u64, email: &str, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(accounts::table.find(id as i64)).set(accounts::email.eq(email))
                                                       .execute(conn)
    }
}

#[derive(Insertable)]
#[diesel(table_name = account_tokens)]
pub struct NewAccountToken<'a> {
    pub account_id: i64,
    pub token:      &'a str,
}

impl AccountToken {
    pub fn list(account_id: u64, conn: &mut PgConnection) -> QueryResult<Vec<AccountToken>> {
        Counter::DBCall.increment();
        account_tokens::table.filter(account_tokens::account_id.eq(account_id as i64))
                             .get_results(conn)
    }

    pub fn create(req: &NewAccountToken, conn: &mut PgConnection) -> QueryResult<AccountToken> {
        Counter::DBCall.increment();
        diesel::insert_into(account_tokens::table).values(req)
                                                  .on_conflict(account_tokens::account_id)
                                                  .do_update()
                                                  .set(account_tokens::token.eq(req.token))
                                                  .get_result(conn)
    }

    pub fn delete(id: u64, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(account_tokens::table.find(id as i64)).execute(conn)
    }
}

#[cfg(test)]
mod oauth_tests {
    use super::*;
    use crate::models::{origin::{NewOrigin,
                                 Origin,
                                 OriginMember},
                        package::PackageVisibility};
    use std::time::{SystemTime,
                    UNIX_EPOCH};

    #[test]
    fn oauth_identity_binds_legacy_and_preserves_ownership() {
        // Run against a migrated disposable database. Every fixture is rolled
        // back, including the legacy row and provider bindings.
        let url = match std::env::var("BUILDER_TEST_DATABASE_URL") {
            Ok(url) => url,
            Err(_) => return,
        };
        let mut conn = PgConnection::establish(&url).unwrap();
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)
                                      .unwrap()
                                      .as_nanos();
        let old_name = format!("oauth_{}_old", suffix);
        let new_name = format!("oauth_{}_new", suffix);
        let collision_name = format!("oauth_{}_collision", suffix);
        let bound_collision_name = format!("oauth_{}_bound_collision", suffix);
        let origin_name = format!("oauth_{}_origin", suffix);
        let backfilled_name = format!("oauth_{}_backfilled", suffix);
        let backfilled_rename = format!("oauth_{}_backfilled_new", suffix);
        let token_value = format!("oauth-test-token-{}", suffix);

        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let legacy = Account::create(&NewAccount { name:  &old_name,
                                                       email: "", },
                                         conn)?;
            let old = NewAccount { name:  &old_name,
                                   email: "", };
            let bound = Account::find_or_create_oauth(&old, "GitHub", "subject-a", conn)?;
            assert_eq!(bound.id, legacy.id);
            assert_eq!(bound.extern_id.as_deref(), Some("subject-a"));
            let visibility = PackageVisibility::Public;
            Origin::create(&NewOrigin { name: &origin_name,
                                        owner_id: bound.id,
                                        default_package_visibility: &visibility, },
                           conn)?;
            let token = AccountToken::create(&NewAccountToken { account_id: bound.id,
                                                                token:      &token_value, },
                                             conn)?;

            // A different provider identity takes the username before the
            // original owner logs in under the new name.
            let reused = Account::find_or_create_oauth(&old, "GitHub", "subject-b", conn)?;
            assert_ne!(reused.id, legacy.id);
            assert_ne!(Account::get_by_id(legacy.id, conn)?.name, old_name);
            assert!(Origin::list(reused.id, conn)?.is_empty());
            assert!(AccountToken::list(reused.id as u64, conn)?.is_empty());

            let renamed = Account::find_or_create_oauth(&NewAccount { name:  &new_name,
                                                                      email: "", },
                                                        "GitHub",
                                                        "subject-a",
                                                        conn)?;
            assert_eq!(renamed.id, legacy.id);
            assert_eq!(Account::get(&new_name, conn)?.id, legacy.id);
            assert_eq!(Origin::get(&origin_name, conn)?.owner_account, new_name);
            assert_eq!(OriginMember::list(&origin_name, conn)?,
                       vec![new_name.clone()]);
            assert_eq!(Origin::list(legacy.id, conn)?.len(), 1);
            assert_eq!(AccountToken::list(legacy.id as u64, conn)?[0].id, token.id);

            let gitlab = Account::find_or_create_oauth(&NewAccount { name:
                                                                         &format!("oauth_{}_gitlab",
                                                                                  suffix),
                                                                     email: "", },
                                                       "GitLab",
                                                       "subject-a",
                                                       conn)?;
            assert_ne!(gitlab.id, legacy.id);

            // An administrator can populate the verified provider identity
            // before rollout; the first login must reuse that account row.
            let backfilled = Account::create(&NewAccount { name:  &backfilled_name,
                                                           email: "", },
                                             conn)?;
            diesel::update(accounts::table.find(backfilled.id))
                .set((accounts::provider.eq(Some("github")),
                      accounts::extern_id.eq(Some("backfilled-subject"))))
                .execute(conn)?;
            let first_login = Account::find_or_create_oauth(&NewAccount { name:  &backfilled_rename,
                                                                          email: "", },
                                                            "github",
                                                            "backfilled-subject",
                                                            conn)?;
            assert_eq!(first_login.id, backfilled.id);
            assert_eq!(Account::get(&backfilled_rename, conn)?.id, backfilled.id);

            let stale = Account::create(&NewAccount { name:  &collision_name,
                                                      email: "", },
                                        conn)?;
            let moved = Account::find_or_create_oauth(&NewAccount { name:  &collision_name,
                                                                    email: "", },
                                                      "GitHub",
                                                      "subject-a",
                                                      conn)?;
            assert_eq!(moved.id, legacy.id);
            assert_ne!(Account::get_by_id(stale.id, conn)?.name, collision_name);
            assert_eq!(Account::get(&collision_name, conn)?.id, legacy.id);

            let holder = Account::find_or_create_oauth(&NewAccount { name:  &bound_collision_name,
                                                                     email: "", },
                                                       "GitHub",
                                                       "subject-c",
                                                       conn)?;
            let moved_again = Account::find_or_create_oauth(&NewAccount { name:
                                                                              &bound_collision_name,
                                                                          email: "", },
                                                            "GitHub",
                                                            "subject-a",
                                                            conn)?;
            assert_eq!(moved_again.id, legacy.id);
            assert_ne!(Account::get_by_id(holder.id, conn)?.name,
                       bound_collision_name);
            assert_eq!(Account::get(&bound_collision_name, conn)?.id, legacy.id);
            Ok(())
        });
    }
}
