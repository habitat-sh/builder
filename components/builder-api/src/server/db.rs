use actix_web::actix::{Actor, Addr, SyncArbiter, SyncContext};
use actix_web::{error, Error};
use db::config::DataStoreCfg;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

type PgPool = Pool<ConnectionManager<PgConnection>>;
type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

pub struct DbExecutor(pub PgPool);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

pub fn init(config: DataStoreCfg) -> Addr<DbExecutor> {
    let manager = ConnectionManager::<PgConnection>::new(config.to_string());
    let pool = Pool::builder()
        .max_size(config.pool_size)
        .build(manager)
        .expect("Failed to create pool.");
    SyncArbiter::start(4, move || DbExecutor(pool.clone()))
}

impl DbExecutor {
    pub fn get_conn(&self) -> Result<PgPooledConnection, Error> {
        self.0.get().map_err(|e| error::ErrorInternalServerError(e))
    }
}
