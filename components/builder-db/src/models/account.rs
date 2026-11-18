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
