use sqlx::Row;

pub mod status {
    pub const PENDING: &str = "pending";
    pub const READY: &str = "ready";
    pub const PROCESSING: &str = "processing";
    pub const DONE: &str = "done";

    pub async fn is_valid(status: &str) -> bool {
        status == PENDING || status == PROCESSING || status == DONE || status == READY
    }
}

pub mod dbtype {
    /// Maps to the songQueue table. Does not contain the data field
    #[derive(Debug, serde::Deserialize, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
    pub struct SongQueue {
        pub id: uuid::Uuid,
        pub filename: String,
        pub status: String,
        pub user_id: uuid::Uuid,
    }
}

pub async fn insert(
    pool: &sqlx::PgPool,
    filename: &String,
    status: &String,
) -> Result<uuid::Uuid, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO "songQueue" (filename, status) VALUES($1, $2) RETURNING id;
        "#,
    )
    .bind(filename)
    .bind(status)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error inserting: {e}");
    });

    match result {
        Ok(row) => {
            let id: uuid::Uuid = row
                .try_get("id")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap();
            Ok(id)
        }
        Err(_err) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn update(
    pool: &sqlx::PgPool,
    data: &Vec<u8>,
    id: &uuid::Uuid,
) -> Result<Vec<u8>, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE "songQueue" SET data = $1 WHERE id = $2 RETURNING data;
        "#,
    )
    .bind(data)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error inserting: {e}");
    });

    match result {
        Ok(row) => Ok(row
            .try_get("data")
            .map_err(|_e| sqlx::Error::RowNotFound)
            .unwrap()),
        Err(_) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn get_most_recent_and_update(
    pool: &sqlx::PgPool,
) -> Result<dbtype::SongQueue, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE "songQueue"
        SET status = $1
        WHERE id = (
            SELECT id FROM "songQueue"
            WHERE status = $2
            ORDER BY id
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        RETURNING id, filename, status, user_id;
        "#,
    )
    .bind(status::PROCESSING)
    .bind(status::READY)
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => {
            let user_id_result = row.try_get("user_id");
            let song_queue = dbtype::SongQueue {
                id: row
                    .try_get("id")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                filename: row
                    .try_get("filename")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                status: row
                    .try_get("status")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                user_id: match user_id_result {
                    Ok(id) => id,
                    Err(_) => uuid::Uuid::nil(),
                },
            };

            Ok(song_queue)
        }
        Err(_err) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn get_status_of_song_queue(
    pool: &sqlx::PgPool,
    id: &uuid::Uuid,
) -> Result<String, sqlx::Error> {
    let result = sqlx::query(
        r#"
        SELECT id, status FROM "songQueue" WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error selecting: {e}");
    });

    match result {
        Ok(row) => Ok(row
            .try_get("status")
            .map_err(|_e| sqlx::Error::RowNotFound)
            .unwrap()),
        Err(_err) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn update_song_queue_status(
    pool: &sqlx::PgPool,
    status: &String,
    id: &uuid::Uuid,
) -> Result<String, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE "songQueue" SET status = $1 WHERE id = $2 RETURNING status;
        "#,
    )
    .bind(status)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error updating record {e}");
    });

    match result {
        Ok(row) => Ok(row
            .try_get("status")
            .map_err(|_e| sqlx::Error::RowNotFound)
            .unwrap()),
        Err(_) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn link_user_id(
    pool: &sqlx::PgPool,
    id: &uuid::Uuid,
    user_id: &uuid::Uuid,
) -> Result<uuid::Uuid, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE "songQueue" SET user_id = $1 WHERE id = $2 RETURNING user_id;
        "#,
    )
    .bind(user_id)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error updating record {e}");
    });

    match result {
        Ok(row) => Ok(row
            .try_get("user_id")
            .map_err(|_e| sqlx::Error::RowNotFound)
            .unwrap()),
        Err(_) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn get_song_queue(
    pool: &sqlx::PgPool,
    id: &uuid::Uuid,
) -> Result<dbtype::SongQueue, sqlx::Error> {
    let result = sqlx::query(
        r#"
        SELECT id, filename, status, user_id FROM "songQueue" WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error querying data: {e}");
    });

    match result {
        Ok(row) => {
            let user_id_result = row.try_get("user_id");
            let song_queue = dbtype::SongQueue {
                id: row
                    .try_get("id")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                filename: row
                    .try_get("filename")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                status: row
                    .try_get("status")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                user_id: match user_id_result {
                    Ok(id) => id,
                    Err(_) => uuid::Uuid::nil(),
                },
            };

            Ok(song_queue)
        }
        Err(_err) => Err(sqlx::Error::RowNotFound),
    }
}
