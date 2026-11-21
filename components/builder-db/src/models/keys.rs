use super::db_id_format;
use crate::{bldr_core::metrics::CounterMetric,
            metrics::Counter,
            schema::key::*};
use chrono::NaiveDateTime;
use diesel::{self,
             pg::PgConnection,
             result::QueryResult,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = origin_public_encryption_keys)]
pub struct OriginPublicEncryptionKey {
    #[serde(with = "db_id_format")]
    pub id:         i64,
    #[serde(with = "db_id_format")]
    pub owner_id:   i64,
    pub name:       String,
    pub revision:   String,
    pub full_name:  String,
    pub body:       String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub origin:     String,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = origin_private_encryption_keys)]
pub struct OriginPrivateEncryptionKey {
    #[serde(with = "db_id_format")]
    pub id:         i64,
    #[serde(with = "db_id_format")]
    pub owner_id:   i64,
    pub name:       String,
    pub revision:   String,
    pub full_name:  String,
    pub body:       String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub origin:     String,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = origin_secret_keys)]
pub struct OriginPrivateSigningKey {
    #[serde(with = "db_id_format")]
    pub id:                 i64,
    #[serde(with = "db_id_format")]
    pub owner_id:           i64,
    pub name:               String,
    pub revision:           String,
    pub full_name:          String,
    pub body:               String,
    pub created_at:         Option<NaiveDateTime>,
    pub updated_at:         Option<NaiveDateTime>,
    pub origin:             String,
    pub encryption_key_rev: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = origin_public_keys)]
pub struct OriginPublicSigningKey {
    #[serde(with = "db_id_format")]
    pub id:         i64,
    #[serde(with = "db_id_format")]
    pub owner_id:   i64,
    pub name:       String,
    pub revision:   String,
    pub full_name:  String,
    pub body:       String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub origin:     String,
}

#[derive(Insertable)]
#[diesel(table_name = origin_public_encryption_keys)]
pub struct NewOriginPublicEncryptionKey<'a> {
    pub owner_id:  i64,
    pub name:      &'a str,
    pub full_name: &'a str,
    pub revision:  &'a str,
    pub body:      &'a str,
    pub origin:    &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = origin_private_encryption_keys)]
pub struct NewOriginPrivateEncryptionKey<'a> {
    pub owner_id:  i64,
    pub name:      &'a str,
    pub full_name: &'a str,
    pub revision:  &'a str,
    pub body:      &'a str,
    pub origin:    &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = origin_secret_keys)]
pub struct NewOriginPrivateSigningKey<'a> {
    pub owner_id:           i64,
    pub name:               &'a str,
    pub full_name:          &'a str,
    pub revision:           &'a str,
    pub body:               &'a str,
    pub origin:             &'a str,
    pub encryption_key_rev: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = origin_public_keys)]
pub struct NewOriginPublicSigningKey<'a> {
    pub owner_id:  i64,
    pub name:      &'a str,
    pub full_name: &'a str,
    pub revision:  &'a str,
    pub body:      &'a str,
    pub origin:    &'a str,
}

impl OriginPublicEncryptionKey {
    pub fn get(origin: &str,
               revision: &str,
               conn: &mut PgConnection)
               -> QueryResult<OriginPublicEncryptionKey> {
        Counter::DBCall.increment();
        origin_public_encryption_keys::table
            .filter(origin_public_encryption_keys::origin.eq(origin))
            .filter(origin_public_encryption_keys::revision.eq(revision))
            .limit(1)
            .order(origin_public_encryption_keys::revision.desc())
            .get_result(conn)
    }

    pub fn create(req: &NewOriginPublicEncryptionKey,
                  conn: &mut PgConnection)
                  -> QueryResult<OriginPublicEncryptionKey> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_public_encryption_keys::table).values(req)
                                                                 .get_result(conn)
    }

    pub fn latest(origin: &str, conn: &mut PgConnection) -> QueryResult<OriginPublicEncryptionKey> {
        Counter::DBCall.increment();
        origin_public_encryption_keys::table
            .filter(origin_public_encryption_keys::origin.eq(origin))
            .limit(1)
            .order(origin_public_encryption_keys::revision.desc())
            .get_result(conn)
    }

    pub fn list(origin: &str,
                conn: &mut PgConnection)
                -> QueryResult<Vec<OriginPublicEncryptionKey>> {
        Counter::DBCall.increment();
        origin_public_encryption_keys::table
            .filter(origin_public_encryption_keys::origin.eq(origin))
            .order(origin_public_encryption_keys::revision.desc())
            .get_results(conn)
    }
}

impl OriginPrivateEncryptionKey {
    pub fn latest(origin: &str,
                  conn: &mut PgConnection)
                  -> QueryResult<OriginPrivateEncryptionKey> {
        Counter::DBCall.increment();
        // This is really latest because you're not allowed to get old keys
        origin_private_encryption_keys::table
            .filter(origin_private_encryption_keys::origin.eq(origin))
            .limit(1)
            .order(origin_private_encryption_keys::full_name.desc())
            .get_result(conn)
    }

    pub fn create(req: &NewOriginPrivateEncryptionKey,
                  conn: &mut PgConnection)
                  -> QueryResult<OriginPrivateEncryptionKey> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_private_encryption_keys::table).values(req)
                                                                  .get_result(conn)
    }
}

impl OriginPublicSigningKey {
    pub fn get(origin: &str,
               revision: &str,
               conn: &mut PgConnection)
               -> QueryResult<OriginPublicSigningKey> {
        Counter::DBCall.increment();
        origin_public_keys::table.filter(origin_public_keys::origin.eq(origin))
                                 .filter(origin_public_keys::revision.eq(revision))
                                 .limit(1)
                                 .order(origin_public_keys::revision.desc())
                                 .get_result(conn)
    }

    pub fn create(req: &NewOriginPublicSigningKey,
                  conn: &mut PgConnection)
                  -> QueryResult<OriginPublicSigningKey> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_public_keys::table).values(req)
                                                      .get_result(conn)
    }

    pub fn latest(origin: &str, conn: &mut PgConnection) -> QueryResult<OriginPublicSigningKey> {
        Counter::DBCall.increment();
        origin_public_keys::table.filter(origin_public_keys::origin.eq(origin))
                                 .limit(1)
                                 .order(origin_public_keys::revision.desc())
                                 .get_result(conn)
    }

    pub fn list(origin: &str, conn: &mut PgConnection) -> QueryResult<Vec<OriginPublicSigningKey>> {
        Counter::DBCall.increment();
        origin_public_keys::table.filter(origin_public_keys::origin.eq(origin))
                                 .order(origin_public_keys::revision.desc())
                                 .get_results(conn)
    }
}

impl OriginPrivateSigningKey {
    pub fn get(origin: &str, conn: &mut PgConnection) -> QueryResult<OriginPrivateSigningKey> {
        Counter::DBCall.increment();
        // This is really latest because you're not allowed to get old keys
        origin_secret_keys::table.filter(origin_secret_keys::origin.eq(origin))
                                 .limit(1)
                                 .order(origin_secret_keys::full_name.desc())
                                 .get_result(conn)
    }

    pub fn create(req: &NewOriginPrivateSigningKey,
                  conn: &mut PgConnection)
                  -> QueryResult<OriginPrivateSigningKey> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_secret_keys::table).values(req)
                                                      .get_result(conn)
    }

    pub fn delete(origin: &str, revision: &str, conn: &mut PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(origin_secret_keys::table
                           .filter(origin_secret_keys::origin.eq(origin))
                           .filter(origin_secret_keys::revision.eq(revision)))
        .execute(conn)
    }

    pub fn update_key(id: i64,
                      body: &str,
                      key_rev: &str,
                      conn: &mut PgConnection)
                      -> QueryResult<OriginPrivateSigningKey> {
        Counter::DBCall.increment();
        diesel::update(origin_secret_keys::table.filter(origin_secret_keys::id.eq(id)))
            .set((
                origin_secret_keys::body.eq(body),
                origin_secret_keys::encryption_key_rev.eq(Some(key_rev)),
            ))
            .get_result(conn)
    }

    // Get values with a null encryption_key_rev, meaning that it's unencrypted.
    // The structure of this may need to be tweaked to do the right thing, but the intent
    // is to use the btree index on id to cause us to start the linear search for null keys
    // at the point we left off, and search in increasing id order
    pub fn list_unencrypted(start: i64,
                            count: i64,
                            conn: &mut PgConnection)
                            -> QueryResult<Vec<OriginPrivateSigningKey>> {
        origin_secret_keys::table.filter(origin_secret_keys::id.ge(start))
                                 .filter(origin_secret_keys::encryption_key_rev.is_null())
                                 .limit(count)
                                 .order(origin_secret_keys::id.asc())
                                 .get_results(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{account::{Account,
                                  NewAccount},
                        origin::{NewOrigin,
                                 Origin},
                        package::PackageVisibility};
    use diesel::{result::Error::NotFound,
                 Connection};

    #[test]
    #[ignore = "requires a running Studio with a migrated disposable database"]
    fn deleting_the_current_secret_key_makes_the_previous_key_current() {
        let mut conn = crate::test::studio_connection();
        let suffix = uuid_suffix();
        let origin = format!("keyrot{}", suffix);

        conn.test_transaction::<_, diesel::result::Error, _>(|conn| {
            let account = Account::create(&NewAccount { name:  &format!("keyrot_{}", suffix),
                                                        email: "", },
                                          conn)?;
            Origin::create(&NewOrigin { name: &origin,
                                        owner_id: account.id,
                                        default_package_visibility: &PackageVisibility::Public, },
                           conn)?;
            for revision in &["20000101000000", "20000102000000"] {
                let full_name = format!("{}-{}", origin, revision);
                OriginPrivateSigningKey::create(&NewOriginPrivateSigningKey {
                        owner_id: account.id,
                        name: &origin,
                        full_name: &full_name,
                        revision,
                        body: "body",
                        origin: &origin,
                        encryption_key_rev: "bldr-20000101000000",
                    },
                                                    conn)?;
            }

            assert_eq!(OriginPrivateSigningKey::get(&origin, conn)?.revision,
                       "20000102000000");
            assert_eq!(OriginPrivateSigningKey::delete(&origin, "20000102000000", conn)?,
                       1);
            assert_eq!(OriginPrivateSigningKey::get(&origin, conn)?.revision,
                       "20000101000000");
            assert_eq!(OriginPrivateSigningKey::delete(&origin, "20000102000000", conn)?,
                       0);
            assert_eq!(OriginPrivateSigningKey::delete(&origin, "20000101000000", conn)?,
                       1);
            assert!(matches!(OriginPrivateSigningKey::get(&origin, conn), Err(NotFound)));
            Ok(())
        });
    }

    fn uuid_suffix() -> u128 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos()
    }
}
