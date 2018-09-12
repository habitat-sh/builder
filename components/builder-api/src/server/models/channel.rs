use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use diesel::sql_types::*;
use diesel::PgConnection;
use server::schema::origin_channels;

#[derive(Debug, Serialize, Queryable)]
pub struct Channel {
    pub id: i64,
    pub origin_id: i64,
    pub owner_id: i64,
    pub name: String,
    pub created_at: Option<NaiveDateTime>,
    pub update_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "origin_channels"]
pub struct NewChannel {
    pub name: String,
    pub owner_id: i64,
    pub origin_id: i64,
}

// TED: Maybe move these into a functions to a module?

sql_function!{
    fn get_origin_channels_for_origin_v2(origin_id: BigInt, include_sandbox_channels: Bool)
        -> (BigInt, BigInt, BigInt, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn get_origin_channel_v1(origin_name: Text, name: Text)
        -> (BigInt, BigInt, BigInt, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn insert_origin_channel_v1(origin_id: BigInt, owner_id: BigInt, name: Text)
        -> (BigInt, BigInt, BigInt, Text, Nullable<Timestamptz>, Nullable<Timestamptz>)
}

sql_function!{
    fn delete_origin_channel_v1(channel_id: BigInt) -> ()
}
impl Channel {
    pub fn insert(channel: &NewChannel, conn: &PgConnection) -> QueryResult<Channel> {
        diesel::select(insert_origin_channel_v1(
            &channel.origin_id,
            &channel.owner_id,
            &channel.name,
        )).get_result(conn)
    }

    pub fn get(origin_name: String, channel_name: String, conn: &PgConnection) -> Channel {
        diesel::select(get_origin_channel_v1(origin_name, channel_name))
            .get_result(conn)
            .unwrap()
    }

    pub fn list(
        origin_id: i64,
        include_sandbox_channels: bool,
        conn: &PgConnection,
    ) -> Vec<Channel> {
        diesel::select(get_origin_channels_for_origin_v2(
            origin_id,
            include_sandbox_channels,
        )).get_results(conn)
            .unwrap()
    }

    pub fn delete(channel: i64, conn: &PgConnection) -> QueryResult<usize> {
        diesel::select(delete_origin_channel_v1(channel)).execute(conn)
    }
}
