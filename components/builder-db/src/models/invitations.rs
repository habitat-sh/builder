use super::db_id_format;
use chrono::NaiveDateTime;
use diesel::{self,
             pg::PgConnection,
             result::QueryResult,
             ExpressionMethods,
             QueryDsl,
             RunQueryDsl};

use crate::schema::{invitation::origin_invitations,
                    member::origin_members};

use crate::{bldr_core::metrics::CounterMetric,
            metrics::Counter};

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
pub struct OriginInvitation {
    #[serde(with = "db_id_format")]
    pub id:           i64,
    pub origin:       String,
    #[serde(with = "db_id_format")]
    pub account_id:   i64,
    pub account_name: String,
    #[serde(with = "db_id_format")]
    pub owner_id:     i64,
    pub ignored:      bool,
    pub created_at:   Option<NaiveDateTime>,
    pub updated_at:   Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "origin_invitations"]
pub struct NewOriginInvitation<'a> {
    pub origin:       &'a str,
    pub account_id:   i64,
    pub account_name: &'a str,
    pub owner_id:     i64,
}

impl OriginInvitation {
    pub fn create(req: &NewOriginInvitation, conn: &PgConnection) -> QueryResult<OriginInvitation> {
        Counter::DBCall.increment();
        diesel::insert_into(origin_invitations::table).values(req)
                                                      .get_result(conn)
    }

    pub fn list_by_origin(origin: &str, conn: &PgConnection) -> QueryResult<Vec<OriginInvitation>> {
        Counter::DBCall.increment();
        origin_invitations::table.filter(origin_invitations::origin.eq(origin))
                                 .get_results(conn)
    }

    pub fn list_by_account(owner_id: u64,
                           conn: &PgConnection)
                           -> QueryResult<Vec<OriginInvitation>> {
        Counter::DBCall.increment();
        origin_invitations::table.filter(origin_invitations::account_id.eq(owner_id as i64))
                                 .filter(origin_invitations::ignored.eq(false))
                                 .get_results(conn)
    }

    pub fn accept(invite_id: u64, ignore: bool, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        let invitation = origin_invitations::table.find(invite_id as i64);
        if ignore {
            return diesel::update(invitation).set(origin_invitations::ignored.eq(ignore))
                                             .execute(conn);
        }

        diesel::insert_into(origin_members::table)
            .values(invitation.select((origin_invitations::account_id, origin_invitations::origin)))
            .into_columns((origin_members::account_id, origin_members::origin))
            .on_conflict_do_nothing()
            .execute(conn)?;

        diesel::delete(invitation).execute(conn)
    }

    pub fn ignore(invite_id: u64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::update(origin_invitations::table.find(invite_id as i64))
            .set(origin_invitations::ignored.eq(true))
            .execute(conn)
    }

    pub fn rescind(invite_id: u64, conn: &PgConnection) -> QueryResult<usize> {
        Counter::DBCall.increment();
        diesel::delete(origin_invitations::table.find(invite_id as i64)).execute(conn)
    }
}
