use actix_web::{actix::Handler, error, Error};
use server::db::DbExecutor;
use server::models::channel::{Channel, CreateChannel, DeleteChannel, GetChannel, ListChannels};
use std::ops::Deref;

impl Handler<ListChannels> for DbExecutor {
    type Result = Result<Vec<Channel>, Error>;
    fn handle(&mut self, channel: ListChannels, _: &mut Self::Context) -> Self::Result {
        Channel::list(channel, self.get_conn()?.deref())
            .map_err(|_| error::ErrorInternalServerError("Error listing channels"))
    }
}

impl Handler<GetChannel> for DbExecutor {
    type Result = Result<Channel, Error>;
    fn handle(&mut self, channel: GetChannel, _: &mut Self::Context) -> Self::Result {
        Channel::get(channel, self.get_conn()?.deref())
            .map_err(|_| error::ErrorInternalServerError("Error fetching channel"))
    }
}

impl Handler<CreateChannel> for DbExecutor {
    type Result = Result<Channel, Error>;
    fn handle(&mut self, channel: CreateChannel, _: &mut Self::Context) -> Self::Result {
        Channel::create(channel, self.get_conn()?.deref())
            .map_err(|_| error::ErrorInternalServerError("Error creating channel"))
    }
}

impl Handler<DeleteChannel> for DbExecutor {
    type Result = Result<(), Error>;
    fn handle(&mut self, channel: DeleteChannel, _: &mut Self::Context) -> Self::Result {
        Channel::delete(channel, self.get_conn()?.deref())
            .map(|_| ())
            .map_err(|_| error::ErrorInternalServerError("Error deleting channel"))
    }
}
