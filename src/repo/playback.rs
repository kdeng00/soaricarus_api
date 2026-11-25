pub mod queue {
    use sqlx::Error as SqlxError;
    use sqlx::postgres::PgRow;
    use sqlx::{PgPool, Row};

    use simodels::playback_queue::PlaybackQueue;

    pub mod item {
        use sqlx::Error as SqlxError;
        use sqlx::postgres::PgRow;
        use sqlx::{PgPool, Row};

        use simodels::playback_queue::PlaybackQueueItem;

        pub async fn create_queue_item(
            pool: &PgPool,
            playback_queue_item: &PlaybackQueueItem,
        ) -> Result<(uuid::Uuid, time::OffsetDateTime), SqlxError> {
            let query = r#"
            INSERT INTO "playbackQueueItem" 
            (queue_index, playback_queue_id, song_id)
            VALUES ($1, $2, $3)
            RETURNING id, date_created;
            "#;

            match sqlx::query(query)
                .bind(playback_queue_item.queue_index)
                .bind(playback_queue_item.playback_queue_id)
                .bind(playback_queue_item.song_id)
                .fetch_one(pool)
                .await
            {
                Ok(row) => {
                    let id = row
                        .try_get("id")
                        .map_err(|_e| SqlxError::RowNotFound)
                        .unwrap();
                    let date_created = row
                        .try_get("date_created")
                        .map_err(|_e| SqlxError::RowNotFound)
                        .unwrap();

                    Ok((id, date_created))
                }
                Err(err) => Err(err),
            }
        }

        pub async fn queue_items_exists(
            pool: &PgPool,
            playback_queue_id: &uuid::Uuid,
        ) -> Result<bool, SqlxError> {
            let query = r#"
                SELECT id FROM "playbackQueueItem"
                WHERE playback_queue_id = $1;
            "#;

            match sqlx::query(query)
                .bind(playback_queue_id)
                .execute(pool)
                .await
            {
                Ok(row) => {
                    let rows_affected = row.rows_affected();
                    Ok(rows_affected > 0)
                }
                Err(err) => Err(err),
            }
        }

        pub async fn clear_queue_items(
            pool: &PgPool,
            playback_queue_id: &uuid::Uuid,
        ) -> Result<(), SqlxError> {
            let query = r#"
            DELETE FROM "playbackQueueItem" WHERE playback_queue_id = $1;
            "#;

            match sqlx::query(query)
                .bind(playback_queue_id)
                .execute(pool)
                .await
            {
                Ok(_r) => Ok(()),
                Err(err) => Err(err),
            }
        }

        pub async fn clear_queue_item(
            pool: &PgPool,
            playback_queue_item_id: &uuid::Uuid,
        ) -> Result<(), SqlxError> {
            let query = r#"
            DELETE FROM "playbackQueueItem" WHERE id = $1;
            "#;

            match sqlx::query(query)
                .bind(playback_queue_item_id)
                .execute(pool)
                .await
            {
                Ok(_r) => Ok(()),
                Err(err) => Err(err),
            }
        }

        pub async fn get_queue_item(
            pool: &PgPool,
            id: &uuid::Uuid,
        ) -> Result<PlaybackQueueItem, SqlxError> {
            let query = r#"
            SELECT id, queue_index, song_id, playback_queue_id, date_created FROM 
            "playbackQueueItem"
            WHERE
            id = $1;
            "#;

            match sqlx::query(query).bind(id).fetch_one(pool).await {
                Ok(row) => parse_row(&row).await,
                Err(err) => Err(err),
            }
        }

        pub async fn update_index(
            pool: &PgPool,
            playback_queue_item: &PlaybackQueueItem,
            index: i32,
        ) -> Result<(), SqlxError> {
            let query = r#"
            UPDATE "playbackQueueItem" SET queue_index = $1
            WHERE
                id = $2
            "#;

            match sqlx::query(query)
                .bind(index)
                .bind(playback_queue_item.id)
                .execute(pool)
                .await
            {
                Ok(r) => {
                    if r.rows_affected() > 0 {
                        Ok(())
                    } else {
                        Err(SqlxError::RowNotFound)
                    }
                }
                Err(err) => Err(err),
            }
        }

        pub async fn get_queue_items(
            pool: &PgPool,
            playback_queue_id: &uuid::Uuid,
        ) -> Result<Vec<PlaybackQueueItem>, SqlxError> {
            let query = r#"
            SELECT id, queue_index, song_id, playback_queue_id, date_created FROM 
            "playbackQueueItem"
            WHERE
            playback_queue_id = $1 ORDER BY queue_index ASC;
            "#;

            match sqlx::query(query)
                .bind(playback_queue_id)
                .fetch_all(pool)
                .await
            {
                Ok(rows) => {
                    let mut items: Vec<PlaybackQueueItem> = Vec::new();

                    for row in rows {
                        let item = match parse_row(&row).await {
                            Ok(item) => item,
                            Err(err) => {
                                return Err(err);
                            }
                        };

                        items.push(item);
                    }

                    Ok(items)
                }
                Err(err) => Err(err),
            }
        }

        async fn parse_row(row: &PgRow) -> Result<PlaybackQueueItem, SqlxError> {
            Ok(PlaybackQueueItem {
                id: row
                    .try_get("id")
                    .map_err(|_e| SqlxError::RowNotFound)
                    .unwrap(),
                queue_index: row
                    .try_get("queue_index")
                    .map_err(|_e| SqlxError::RowNotFound)
                    .unwrap(),
                playback_queue_id: row
                    .try_get("playback_queue_id")
                    .map_err(|_e| SqlxError::RowNotFound)
                    .unwrap(),
                song_id: row
                    .try_get("song_id")
                    .map_err(|_e| SqlxError::RowNotFound)
                    .unwrap(),
                date_created: row
                    .try_get("date_created")
                    .map_err(|_e| SqlxError::RowNotFound)
                    .unwrap(),
            })
        }
    }

    pub async fn create_queue(
        pool: &PgPool,
        playback_queue: &PlaybackQueue,
    ) -> Result<(uuid::Uuid, time::OffsetDateTime), SqlxError> {
        let query = r#"
        INSERT INTO "playbackQueue" (name, device_type, user_id)
        VALUES ($1, $2, $3)
        RETURNING date_created, id;
        "#;

        match sqlx::query(query)
            .bind(&playback_queue.name)
            .bind(&playback_queue.device_type)
            .bind(playback_queue.user_id)
            .fetch_one(pool)
            .await
        {
            Ok(row) => {
                let id = row
                    .try_get("id")
                    .map_err(|_e| SqlxError::RowNotFound)
                    .unwrap();
                let date_created = row
                    .try_get("date_created")
                    .map_err(|_e| SqlxError::RowNotFound)
                    .unwrap();

                Ok((id, date_created))
            }
            Err(err) => Err(err),
        }
    }

    pub async fn queue_existence_search(
        pool: &PgPool,
        device_type: &str,
        user_id: &uuid::Uuid,
    ) -> Result<bool, SqlxError> {
        let query = r#"
            SELECT id FROM "playbackQueue"
            WHERE device_type = $1 AND user_id = $2;
        "#;

        match sqlx::query(query)
            .bind(device_type)
            .bind(user_id)
            .execute(pool)
            .await
        {
            Ok(row) => {
                let rows_affected = row.rows_affected();
                Ok(rows_affected > 0)
            }
            Err(err) => Err(err),
        }
    }

    pub async fn get(pool: &PgPool, id: &uuid::Uuid) -> Result<PlaybackQueue, SqlxError> {
        let query = r#"
            SELECT id, name, device_type, user_id, date_created 
            FROM "playbackQueue"
            WHERE id = $1
        "#;

        match sqlx::query(query).bind(id).fetch_one(pool).await {
            Ok(row) => parse_row(&row).await,
            Err(err) => Err(err),
        }
    }

    pub async fn get_with_device_type_and_user_id(
        pool: &PgPool,
        device_type: &str,
        user_id: &uuid::Uuid,
    ) -> Result<PlaybackQueue, SqlxError> {
        let query = r#"
            SELECT id, name, device_type, user_id, date_created 
            FROM "playbackQueue"
            WHERE device_type = $1 AND user_id = $2
        "#;

        match sqlx::query(query)
            .bind(device_type)
            .bind(user_id)
            .fetch_one(pool)
            .await
        {
            Ok(row) => parse_row(&row).await,
            Err(err) => Err(err),
        }
    }

    async fn parse_row(row: &PgRow) -> Result<PlaybackQueue, SqlxError> {
        Ok(PlaybackQueue {
            id: row
                .try_get("id")
                .map_err(|_e| SqlxError::RowNotFound)
                .unwrap(),
            name: row
                .try_get("name")
                .map_err(|_e| SqlxError::RowNotFound)
                .unwrap(),
            device_type: row
                .try_get("device_type")
                .map_err(|_e| SqlxError::RowNotFound)
                .unwrap(),
            user_id: row
                .try_get("user_id")
                .map_err(|_e| SqlxError::RowNotFound)
                .unwrap(),
            date_created: row
                .try_get("date_created")
                .map_err(|_e| SqlxError::RowNotFound)
                .unwrap(),
        })
    }
}
