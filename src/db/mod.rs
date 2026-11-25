use sqlx::Error as SqlxError;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub mod connection_settings {
    pub const MAXCONN: u32 = 10;
}

pub async fn create_pool() -> Result<PgPool, SqlxError> {
    let database_url = sienvy::environment::get_db_url().value;

    PgPoolOptions::new()
        .max_connections(connection_settings::MAXCONN)
        .connect(&database_url)
        .await
}

pub async fn migrations(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");
}
