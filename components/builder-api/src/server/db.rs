use actix_web::actix::{Actor, Addr, SyncArbiter, SyncContext};
use db::config::DataStoreCfg;
use diesel::prelude::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2::Pool;

pub struct DbPool(pub Pool<ConnectionManager<PgConnection>>);

impl Actor for DbPool {
    type Context = SyncContext<Self>;
}

pub fn init(config: DataStoreCfg) -> Addr<DbPool> {
    let manager = ConnectionManager::<PgConnection>::new(config.to_string());
    let conn = Pool::builder()
        .max_size(1)
        .build(manager)
        .expect("Failed to create pool.");
    SyncArbiter::start(4, move || DbPool(conn.clone()))
}
