use db::config::DataStoreCfg;
use diesel::prelude::PgConnection;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;

pub struct DbPool(pub Pool<ConnectionManager<PgConnection>>);

pub fn init(config: DataStoreCfg) -> DbPool {
    let manager = ConnectionManager::<PgConnection>::new(config.to_string());
    let conn = Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    DbPool(conn.clone())
}
