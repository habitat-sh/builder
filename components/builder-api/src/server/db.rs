use actix_web::{error, Error};
use db::config::DataStoreCfg;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

type PgPool = Pool<ConnectionManager<PgConnection>>;
type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct DbPool(pub PgPool);

pub fn init(config: DataStoreCfg) -> DbPool {
    DbPool(
        Pool::builder()
            .max_size(config.pool_size)
            .build(ConnectionManager::<PgConnection>::new(config.to_string()))
            .expect("Failed to create pool."),
    )
}

impl DbPool {
    pub fn get_conn(&self) -> Result<PgPooledConnection, Error> {
        self.0.get().map_err(|e| error::ErrorInternalServerError(e))
    }
}
