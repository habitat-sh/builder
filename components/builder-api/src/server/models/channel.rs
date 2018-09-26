use actix_web::{actix::Message, Error};
use chrono::NaiveDateTime;
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

pub struct ChannelList {
    pub origin: String,
    pub include_sandbox_channels: bool,
}

pub struct ChannelGet {
    pub origin: String,
    pub channel: String,
}

impl Message for NewChannel {
    type Result = Result<Channel, Error>;
}

impl Message for ChannelList {
    type Result = Result<Vec<Channel>, Error>;
}

impl Message for ChannelGet {
    type Result = Result<Channel, Error>;
}
