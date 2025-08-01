use super::db_id_format;
use chrono::NaiveDateTime;
use diesel::{self,
             dsl::count,
             pg::{Pg,
                  PgConnection},
             prelude::*,
             result::{Error,
                      QueryResult},
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};

use crate::models::{channel::{Channel,
                              CreateChannel},
                    package::PackageVisibility};

use crate::schema::{audit::audit_origin,
                    channel::origin_channels,
                    integration::origin_integrations,
                    invitation::origin_invitations,
                    key::{origin_private_encryption_keys,
                          origin_public_encryption_keys,
                          origin_public_keys,
                          origin_secret_keys},
                    member::origin_members,
                    origin::{origins,
                             origins_with_secret_key,
                             origins_with_stats},
                    package::origin_packages,
                    project::origin_projects,
                    project_integration::origin_project_integrations,
                    secrets::origin_secrets,
                    settings::origin_package_settings};

use crate::{bldr_core::{metrics::CounterMetric,
                        Error as BuilderError},
            hab_core::ChannelIdent,
            metrics::Counter};

use std::{fmt,
          str::FromStr};

use diesel_derive_enum::DbEnum;

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = origins)]
pub struct Origin {
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub default_package_visibility: PackageVisibility,
}

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct OriginWithSecretKey {
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub private_key_name: Option<String>,
    pub default_package_visibility: PackageVisibility,
    pub owner_account: String,
}

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct OriginWithStats {
    #[serde(with = "db_id_format")]
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub default_package_visibility: PackageVisibility,
    pub package_count: i64,
}

#[derive(Clone,
         Copy,
         DbEnum,
         Debug,
         Serialize,
         Deserialize,
         PartialEq,
         PartialOrd)]
#[ExistingTypePath = "crate::schema::sql_types::OriginMemberRole"]
#[DbValueStyle = "snake_case"]
pub enum OriginMemberRole {
    // It is important to preserve the declaration order
    // here so that order comparisons work as expected.
    // The values are from least to greatest.
    #[serde(rename = "readonly_member")]
    ReadonlyMember,
    #[serde(rename = "member")]
    Member,
    #[serde(rename = "maintainer")]
    Maintainer,
    #[serde(rename = "administrator")]
    Administrator,
    #[serde(rename = "owner")]
    Owner,
}

impl fmt::Display for OriginMemberRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match *self {
            OriginMemberRole::Administrator => "administrator",
            OriginMemberRole::Maintainer => "maintainer",
            OriginMemberRole::Member => "member",
            OriginMemberRole::Owner => "owner",
            OriginMemberRole::ReadonlyMember => "readonly_member",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for OriginMemberRole {
    type Err = BuilderError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            "administrator" => Ok(OriginMemberRole::Administrator),
            "maintainer" => Ok(OriginMemberRole::Maintainer),
            "member" => Ok(OriginMemberRole::Member),
            "owner" => Ok(OriginMemberRole::Owner),
            "readonly_member" => Ok(OriginMemberRole::ReadonlyMember),
            _ => {
                Err(BuilderError::OriginMemberRoleError(format!(
                    "Invalid OriginMemberRole \"{}\", must be one of: [\"administrator\", \
                     \"maintainer\",\"member\",\"owner\",\"readonly_member\"].",
                    value
                )))
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Queryable, QueryableByName, Insertable)]
#[diesel(table_name = origin_members)]
pub struct OriginMember {
    #[serde(with = "db_id_format")]
    pub account_id:  i64,
    pub origin:      String,
    pub member_role: OriginMemberRole,
    pub created_at:  Option<NaiveDateTime>,
    pub updated_at:  Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = origins)]
pub struct NewOrigin<'a> {
    pub name: &'a str,
    pub owner_id: i64,
    pub default_package_visibility: &'a PackageVisibility,
}

#[derive(Clone, Copy, DbEnum, Debug, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::OriginOperation"]
#[DbValueStyle = "snake_case"]
pub enum OriginOperation {
    OriginCreate,
    OriginDelete,
    OwnerTransfer,
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = audit_origin)]
struct OriginAudit<'a> {
    operation:      OriginOperation,
    origin:         &'a str,
    requester_id:   i64,
    requester_name: &'a str,
    target_object:  &'a str,
}

impl OriginAudit<'_> {
    fn audit(origin_audit_record: &OriginAudit, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::insert_into(audit_origin::table).values(origin_audit_record)
                                                .execute(conn)
    }
}

pub fn origin_audit(origin: &str,
                    op: OriginOperation,
                    target: &str,
                    id: i64,
                    name: &str,
                    conn: &mut PgConnection) {
    if let Err(err) = OriginAudit::audit(&OriginAudit { operation: op,
                                                        origin,
                                                        target_object: target,
                                                        requester_id: id,
                                                        requester_name: name },
                                         conn)
    {
        debug!("Failed to save origin [{}] {:?} operation to audit log: {}",
               origin, op, err);
    }
}

impl Origin {
    pub fn get(origin: &str, conn: &mut PgConnection) -> QueryResult<OriginWithSecretKey> {
        Counter::DBCall.increment();
        origins_with_secret_key::table.find(origin).first(conn)
    }

    pub fn list(owner_id: i64, conn: &mut PgConnection) -> QueryResult<Vec<OriginWithStats>> {
        Counter::DBCall.increment();
        origins_with_stats::table.into_boxed::<Pg>()
                                 .inner_join(origin_members::table)
                                 .filter(origin_members::account_id.eq(owner_id))
                                 .order(origins_with_stats::name.asc())
                                 .select(origins_with_stats::table::all_columns())
                                 .load(conn)
    }

    pub fn create(req: &NewOrigin, conn: &mut PgConnection) -> QueryResult<Origin> {
        Counter::DBCall.increment();
        let new_origin = diesel::insert_into(origins::table).values(req)
                                                            .get_result(conn)?;

        OriginMember::add(req.name, req.owner_id, conn, OriginMemberRole::Owner)?;
        Channel::create(&CreateChannel { name:     ChannelIdent::unstable().as_str(),
                                         owner_id: req.owner_id,
                                         origin:   req.name, },
                        conn)?;
        Channel::create(&CreateChannel { name:     ChannelIdent::stable().as_str(),
                                         owner_id: req.owner_id,
                                         origin:   req.name, },
                        conn)?;

        Ok(new_origin)
    }

    pub fn update(name: &str,
                  dpv: PackageVisibility,
                  conn: &mut PgConnection)
                  -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origins::table.find(name)).set(origins::default_package_visibility.eq(dpv))
                                                 .execute(conn)
    }

    pub fn delete(origin: &str, conn: &mut PgConnection) -> QueryResult<()> {
        // By this point, most of the associated origin data has already been manually deleted
        // by the user. We ensure this by double checking the most critical tables are already empty
        // via builder_api::server::resources::origins::origin_delete_preflight
        // The transaction entries below explicitly enumerate deletion of all table data associated
        // with the origin to ensure no vestigial data remains.

        Counter::DBCall.increment();
        conn.transaction::<_, Error, _>(|txn_conn| {
            diesel::delete(origin_channels::table.filter(origin_channels::origin.eq(origin)))
                .execute(txn_conn)?;
            diesel::delete(origin_secret_keys::table.filter(origin_secret_keys::origin.eq(origin)))
                .execute(txn_conn)?;
            diesel::delete(origin_public_keys::table.filter(origin_public_keys::origin.eq(origin)))
                .execute(txn_conn)?;
            diesel::delete(origin_members::table.filter(origin_members::origin.eq(origin)))
                .execute(txn_conn)?;
            diesel::delete(
                origin_package_settings::table.filter(origin_package_settings::origin.eq(origin)),
            )
            .execute(txn_conn)?;
            diesel::delete(
                origin_integrations::table.filter(origin_integrations::origin.eq(origin)),
            )
            .execute(txn_conn)?;
            diesel::delete(origin_invitations::table.filter(origin_invitations::origin.eq(origin)))
                .execute(txn_conn)?;
            diesel::delete(
                origin_project_integrations::table
                    .filter(origin_project_integrations::origin.eq(origin)),
            )
            .execute(txn_conn)?;
            diesel::delete(origin_projects::table.filter(origin_projects::origin.eq(origin)))
                .execute(txn_conn)?;
            diesel::delete(origin_secrets::table.filter(origin_secrets::origin.eq(origin)))
                .execute(txn_conn)?;
            diesel::delete(
                origin_private_encryption_keys::table
                    .filter(origin_private_encryption_keys::origin.eq(origin)),
            )
            .execute(txn_conn)?;
            diesel::delete(
                origin_public_encryption_keys::table
                    .filter(origin_public_encryption_keys::origin.eq(origin)),
            )
            .execute(txn_conn)?;
            diesel::delete(origin_packages::table.filter(origin_packages::origin.eq(origin)))
                .execute(txn_conn)?;
            diesel::delete(origins::table.filter(origins::name.eq(origin))).execute(txn_conn)?;
            Ok(())
        })
    }

    pub fn transfer(origin: &str, account_id: i64, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        conn.transaction::<_, Error, _>(|txn_conn| {
                let owner = OriginMemberRole::Owner;
                let maintainer = OriginMemberRole::Maintainer;

                diesel::update(origin_members::table.filter(origin_members::origin.eq(&origin)))
                .filter(origin_members::member_role.eq(owner))
                .set(origin_members::member_role.eq(maintainer))
                .execute(txn_conn)?;

                diesel::update(origin_members::table.filter(origin_members::origin.eq(&origin)))
                .filter(origin_members::account_id.eq(account_id))
                .set(origin_members::member_role.eq(owner))
                .execute(txn_conn)?;

                diesel::update(origins::table.find(origin)).set(origins::owner_id.eq(account_id))
                                                           .execute(txn_conn)
            })
    }

    pub fn depart(origin: &str, account_id: i64, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(
            origin_members::table
                .filter(origin_members::account_id.eq(account_id))
                .filter(origin_members::origin.eq(origin)),
        )
        .execute(conn)
    }

    pub fn check_membership(origin: &str,
                            account_id: i64,
                            conn: &mut PgConnection)
                            -> QueryResult<bool> {
        Counter::DBCall.increment();
        origin_members::table.filter(origin_members::origin.eq(origin))
                             .filter(origin_members::account_id.eq(account_id))
                             .execute(conn)
                             .map(|s| (s > 0))
    }
}

impl OriginMember {
    pub fn list(origin: &str, conn: &mut PgConnection) -> QueryResult<Vec<String>> {
        use crate::schema::account::accounts;

        Counter::DBCall.increment();
        origin_members::table.inner_join(accounts::table)
                             .select(accounts::name)
                             .filter(origin_members::origin.eq(origin))
                             .order(accounts::name.asc())
                             .get_results(conn)
    }

    pub fn delete(origin: &str, account_name: &str, conn: &mut PgConnection) -> QueryResult<usize> {
        use crate::schema::account::accounts;

        Counter::DBCall.increment();
        diesel::delete(
            origin_members::table
                .filter(origin_members::origin.eq(origin))
                .filter(
                    origin_members::account_id.nullable().eq(accounts::table
                        .select(accounts::id)
                        .filter(accounts::name.eq(account_name))
                        .single_value()),
                ),
        )
        .execute(conn)
    }

    pub fn add(origin: &str,
               account_id: i64,
               conn: &mut PgConnection,
               member_role: OriginMemberRole)
               -> QueryResult<usize> {
        diesel::insert_into(origin_members::table)
            .values((
                origin_members::origin.eq(origin),
                origin_members::account_id.eq(account_id),
                origin_members::member_role.eq(member_role),
            ))
            .execute(conn)
    }

    pub fn count_origin_members(origin: &str, conn: &mut PgConnection) -> QueryResult<i64> {
        Counter::DBCall.increment();
        origin_members::table.select(count(origin_members::account_id))
                             .filter(origin_members::origin.eq(&origin))
                             .first(conn)
    }

    pub fn member_role(origin: &str,
                       account_id: i64,
                       conn: &mut PgConnection)
                       -> QueryResult<OriginMemberRole> {
        Counter::DBCall.increment();
        origin_members::table.select(origin_members::member_role)
                             .filter(origin_members::origin.eq(&origin))
                             .filter(origin_members::account_id.eq(account_id))
                             .get_result(conn)
    }

    pub fn update_member_role(origin: &str,
                              account_id: i64,
                              conn: &mut PgConnection,
                              member_role: OriginMemberRole)
                              -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_members::table.filter(origin_members::origin.eq(&origin)))
            .filter(origin_members::account_id.eq(account_id))
            .set(origin_members::member_role.eq(member_role))
            .execute(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_member_role_hierarchy() {
        let readonly_member = OriginMemberRole::ReadonlyMember;
        let member = OriginMemberRole::Member;
        let maintainer = OriginMemberRole::Maintainer;
        let administrator = OriginMemberRole::Administrator;
        let owner = OriginMemberRole::Owner;
        assert!(owner > administrator);
        assert!(administrator > maintainer);
        assert!(maintainer > member);
        assert!(member > readonly_member);
    }
}
