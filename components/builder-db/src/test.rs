use std::sync::{atomic::AtomicUsize,
                Once};

pub static INIT_TEMPLATE: Once = Once::new();
pub static TEST_COUNT: AtomicUsize = AtomicUsize::new(0);

pub mod postgres {
    use std::{path::PathBuf,
              process::{Child,
                        Command,
                        Stdio},
              sync::Once,
              thread};

    struct Postgres {
        inner: Child,
    }

    static POSTGRES: Once = Once::new();

    pub fn start() {
        POSTGRES.call_once(|| {
                    thread::spawn(move || {
                        let mut postgres = Postgres::new();
                        let _ = postgres.inner.wait();
                    });
                });
        std::thread::sleep(std::time::Duration::from_secs(4));
    }

    impl Postgres {
        fn new() -> Postgres {
            let (stdin, stdout, stderr) = if std::env::var("DEBUG").is_ok() {
                (Stdio::inherit(), Stdio::inherit(), Stdio::inherit())
            } else {
                (Stdio::null(), Stdio::null(), Stdio::null())
            };
            // debug should be Stdio::inherit();
            let root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
                                                                     .join("db");
            let start_path = root_path.join("start.sh");
            let child = Command::new("sudo").arg("-E")
                                            .arg(start_path)
                                            .stdin(stdin)
                                            .stdout(stdout)
                                            .stderr(stderr)
                                            .env("DB_TEST_DIR", root_path)
                                            .current_dir("/tmp")
                                            .spawn()
                                            .expect("Failed to launch core/postgresql");
            Postgres { inner: child }
        }
    }
}

#[macro_export]
macro_rules! datastore_test {
    ($datastore:ident) => {{
        use std::sync::{atomic::Ordering,
                        Arc};
        use $crate::{config::DataStoreCfg,
                     diesel_pool::DbPool,
                     pool::Pool,
                     test::{postgres,
                            INIT_TEMPLATE,
                            TEST_COUNT}};

        postgres::start();

        let mut config = DataStoreCfg::default();
        let db_template = "builder_db_test_template";

        // Could be timing problems
        // Ideally, we'd get postgres set up correctly, but that is eluding me right now
        let mut passwd = std::fs::read_to_string("/hab/svc/postgresql/config/pwfile").ok();

        INIT_TEMPLATE.call_once(|| {
                         // Use template1 to create our pool and recreate our test database
                         config.database = "template1".to_string();
                         config.user = "admin".to_string();
                         config.password = passwd.clone();
                         config.pool_size = 1;
                         config.connection_timeout_sec = 5;
                         let pool = Pool::new(&config);
                         let conn = pool.get().expect("Failed to get connection");

                         conn.execute(format!("DROP DATABASE IF EXISTS {}", db_template).as_str(),
                                      &[])
                             .expect("Failed to drop existing template database");
                         conn.execute(format!("CREATE DATABASE {}", db_template).as_str(), &[])
                             .expect("Failed to create template database");

                         // Now that the database is recreated, set config to use that database
                         config.database = db_template.to_string();

                         let diesel_pool = DbPool::new(&config);
                         let template_pool = Pool::new(&config);

                         // Run any builder-db migrations
                         let _ =
                             habitat_builder_db::migration::setup(&diesel_pool.get_conn().unwrap());

                         // Run jobsrv migrations
                         let ds = $datastore::from_pool(template_pool, diesel_pool.clone());
                         ds.setup().expect("Failed to migrate data");
                     });

        println!("Finished db migrations");

        let test_number = TEST_COUNT.fetch_add(1, Ordering::SeqCst);
        let db_name = format!("builder_db_test_{}", test_number);

        let mut config = DataStoreCfg::default();
        config.database = "template1".to_string();
        config.user = "admin".to_string();
        config.password = passwd.clone();
        config.pool_size = 1;
        config.connection_timeout_sec = 5;
        let create_pool = Pool::new(&config);
        let conn = create_pool.get().expect("Failed to get connection");
        let drop_db = format!("DROP DATABASE IF EXISTS {}", db_name);
        let create_db = format!("CREATE DATABASE {} TEMPLATE {}", db_name, db_template);
        conn.execute(&drop_db, &[])
            .expect("Failed to drop test database");
        conn.execute(&create_db, &[])
            .expect("Failed to create test database from template");

        config.database = db_name;
        config.pool_size = 5;
        let diesel_pool = DbPool::new(&config);
        let pool = Pool::new(&config);

        $datastore::from_pool(pool, diesel_pool)
    }};
}
