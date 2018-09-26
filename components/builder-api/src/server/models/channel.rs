use actix_web::{actix::Message, Error};
use chrono::NaiveDateTime;
use diesel;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use diesel::sql_types::{BigInt, Bool, Text};
use diesel::RunQueryDsl;
use server::schema::channel::*;

#[derive(Debug, Serialize, QueryableByName)]
#[table_name = "origin_channels"]
pub struct Channel {
    pub id: i64,
    pub origin_id: i64,
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "origin_channels"]
pub struct NewChannel {
    pub name: String,
    pub owner_id: i64,
    pub origin_id: i64,
}

pub struct ListChannels {
    pub origin: String,
    pub include_sandbox_channels: bool,
}

pub struct GetChannel {
    pub origin: String,
    pub channel: String,
}

pub struct DeleteChannel {
    pub origin: String,
    pub channel: String,
}

impl Message for NewChannel {
    type Result = Result<Channel, Error>;
}

impl Message for ListChannels {
    type Result = Result<Vec<Channel>, Error>;
}

impl Message for GetChannel {
    type Result = Result<Channel, Error>;
}

impl Message for DeleteChannel {
    type Result = Result<(), Error>;
}

impl Channel {
    pub fn list(channel: ListChannels, conn: &PgConnection) -> QueryResult<Vec<Channel>> {
        diesel::sql_query("select * from get_origin_channels_for_origin_v3($1, $2)")
            .bind::<Text, _>(channel.origin)
            .bind::<Bool, _>(channel.include_sandbox_channels)
            .get_results(conn)
    }

    pub fn get(channel: GetChannel, conn: &PgConnection) -> QueryResult<Channel> {
        diesel::sql_query("select * from get_origin_channel_v1($1, $2)")
            .bind::<Text, _>(channel.origin)
            .bind::<Text, _>(channel.channel)
            .get_result(conn)
    }

    pub fn insert(channel: NewChannel, conn: &PgConnection) -> QueryResult<Channel> {
        diesel::sql_query("select * from insert_origin_channel_v1($1, $2, $3)")
            .bind::<BigInt, _>(channel.origin_id)
            .bind::<BigInt, _>(channel.owner_id)
            .bind::<Text, _>(channel.name)
            .get_result(conn)
    }

    pub fn delete(channel: DeleteChannel, conn: &PgConnection) -> QueryResult<usize> {
        diesel::sql_query("select * from delete_origin_channel_v2($1, $2)")
            .bind::<Text, _>(channel.channel)
            .bind::<Text, _>(channel.origin)
            .execute(conn)
    }
}
