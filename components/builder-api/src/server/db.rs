use db::config::DataStoreCfg;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use server::error::Result;

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
    pub fn get_conn(&self) -> Result<PgPooledConnection> {
        match self.0.get() {
            Ok(conn) => Ok(conn),
            Err(e) => Err(e.into()),
        }
    }
}
