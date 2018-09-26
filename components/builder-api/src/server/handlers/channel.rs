use actix_web::{actix::Handler, error, Error};
use diesel;
use diesel::sql_types::{Bool, Text};
use diesel::RunQueryDsl;
use server::db::DbPool;
use server::models::channel::{Channel, ChannelGet, ChannelList};

impl Handler<ChannelList> for DbPool {
    type Result = Result<Vec<Channel>, Error>;
    fn handle(&mut self, channel_list: ChannelList, _: &mut Self::Context) -> Self::Result {
        let conn = &self.0.get().map_err(error::ErrorInternalServerError)?;
        let channels = diesel::sql_query("select * from get_origin_channels_for_origin_v3($1, $2)")
            .bind::<Text, _>(channel_list.origin)
            .bind::<Bool, _>(channel_list.include_sandbox_channels)
            .get_results(conn)
            .unwrap();

        Ok(channels)
    }
}

impl Handler<ChannelGet> for DbPool {
    type Result = Result<Channel, Error>;
    fn handle(&mut self, channel_get: ChannelGet, _: &mut Self::Context) -> Self::Result {
        let conn = &self.0.get().map_err(error::ErrorInternalServerError)?;
        let channel =
            diesel::sql_query("select * from get_origin_channel_v1(origin_name, channel_name)")
                .bind::<Text, _>(channel_get.origin)
                .bind::<Text, _>(channel_get.channel)
                .get_result(conn)
                .unwrap();

        Ok(channel)
    }
}
