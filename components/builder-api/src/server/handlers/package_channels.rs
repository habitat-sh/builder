use actix_web::{actix::Handler, error, Error};
use server::db::DbExecutor;
use server::models::package_channels::{OriginChannelPackage, PromotePackage};
use std::ops::Deref;

impl Handler<PromotePackage> for DbExecutor {
    type Result = Result<(), Error>;
    fn handle(&mut self, package: PromotePackage, _: &mut Self::Context) -> Self::Result {
        OriginChannelPackage::promote(package, self.get_conn()?.deref())
            .map(|_| ())
            .map_err(|_| error::ErrorInternalServerError("Error promoting package"))
    }
}
