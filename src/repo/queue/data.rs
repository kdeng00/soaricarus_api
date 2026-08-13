use sqlx::Row;

pub async fn insert(
    pool: &sqlx::PgPool,
    file_key: &str,
    bucket: &str,
    region: &str,
    song_queue_id: &uuid::Uuid,
) -> Result<uuid::Uuid, sqlx::Error> {
    match sqlx::query(
        r#"
        INSERT INTO "songQueueData" (file_key, bucket, region, song_queue_id) VALUES($1, $2, $3, $4) RETURNING id;
        "#,
    )
    .bind(file_key)
    .bind(bucket)
    .bind(region)
    .bind(song_queue_id)
    .fetch_one(pool).await {
        Ok(row) => {
            let id: uuid::Uuid = row.try_get("id")?;
            Ok(id)
        }
        Err(_err) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn update_file_key(
    pool: &sqlx::PgPool,
    id: &uuid::Uuid,
    file_key: &str,
) -> Result<(), sqlx::Error> {
    match sqlx::query(
        r#"
        UPDATE "songQueueData" SET file_key = $1 WHERE id = $2;
        "#,
    )
    .bind(file_key)
    .bind(id)
    .execute(pool)
    .await
    {
        Ok(_row) => Ok(()),
        Err(_) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn get(
    pool: &sqlx::PgPool,
    id: &uuid::Uuid,
) -> Result<(uuid::Uuid, String, String, String, uuid::Uuid), sqlx::Error> {
    match sqlx::query(
        r#"
        SELECT id, file_key, bucket, region, song_queue_id FROM "songQueueData"
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    {
        Ok(row) => {
            let file_key: String = row.try_get("file_key")?;
            let bucket: String = row.try_get("bucket")?;
            let region: String = row.try_get("region")?;
            let song_queue_id: uuid::Uuid = row.try_get("song_queue_id")?;

            Ok((*id, file_key, bucket, region, song_queue_id))
        }
        Err(err) => Err(err),
    }
}

pub async fn get_with_song_queue_id(
    pool: &sqlx::PgPool,
    song_queue_id: &uuid::Uuid,
) -> Result<(uuid::Uuid, String, String, String, uuid::Uuid), sqlx::Error> {
    match sqlx::query(
        r#"
        SELECT id, file_key, bucket, region, song_queue_id FROM "songQueueData"
        WHERE song_queue_id = $1
        "#,
    )
    .bind(song_queue_id)
    .fetch_one(pool)
    .await
    {
        Ok(row) => {
            let id: uuid::Uuid = row.try_get("id")?;
            let file_key: String = row.try_get("file_key")?;
            let bucket: String = row.try_get("bucket")?;
            let region: String = row.try_get("region")?;

            Ok((id, file_key, bucket, region, *song_queue_id))
        }
        Err(err) => Err(err),
    }
}
