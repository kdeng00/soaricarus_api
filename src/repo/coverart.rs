use sqlx::Row;

pub async fn create(
    pool: &sqlx::PgPool,
    coverart: &simodels::coverart::CoverArt,
    song_id: &uuid::Uuid,
) -> Result<uuid::Uuid, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO "coverart" (title, directory, filename, file_type, song_id) VALUES($1, $2, $3, $4, $5) RETURNING id;
        "#,
    )
    .bind(&coverart.title)
    .bind(&coverart.directory)
    .bind(&coverart.filename)
    .bind(&coverart.file_type)
    .bind(song_id)
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

pub async fn get_coverart(
    pool: &sqlx::PgPool,
    id: &uuid::Uuid,
) -> Result<simodels::coverart::CoverArt, sqlx::Error> {
    let result = sqlx::query(
        r#"
        SELECT id, title, directory, filename, file_type, song_id FROM "coverart" WHERE id = $1;
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error querying data: {e:?}");
    });

    match result {
        Ok(row) => Ok(simodels::coverart::CoverArt {
            id: row
                .try_get("id")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            title: row
                .try_get("title")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            directory: row
                .try_get("directory")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            filename: row
                .try_get("filename")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            file_type: row
                .try_get("file_type")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            song_id: row
                .try_get("song_id")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            ..Default::default()
        }),
        Err(_) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn get_coverart_with_song_id(
    pool: &sqlx::PgPool,
    song_id: &uuid::Uuid,
) -> Result<simodels::coverart::CoverArt, sqlx::Error> {
    let result = sqlx::query(
        r#"
        SELECT id, title, directory, filename, file_type, song_id FROM "coverart" WHERE song_id = $1;
        "#,
    )
    .bind(song_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error querying data: {e:?}");
    });

    match result {
        Ok(row) => Ok(simodels::coverart::CoverArt {
            id: row
                .try_get("id")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            title: row
                .try_get("title")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            directory: row
                .try_get("directory")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            filename: row
                .try_get("filename")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            file_type: row
                .try_get("file_type")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            data: Vec::new(),
            song_id: row
                .try_get("song_id")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
        }),
        Err(_) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn delete_coverart(
    pool: &sqlx::PgPool,
    id: &uuid::Uuid,
) -> Result<simodels::coverart::CoverArt, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM "coverart"
        WHERE id = $1
        RETURNING id, title, directory, filename, file_type, song_id
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error deleting data: {e:?}");
    });

    match result {
        Ok(row) => Ok(simodels::coverart::CoverArt {
            id: row
                .try_get("id")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            title: row
                .try_get("title")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            directory: row
                .try_get("directory")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            filename: row
                .try_get("filename")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            file_type: row
                .try_get("file_type")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            song_id: row
                .try_get("song_id")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap(),
            data: Vec::new(),
        }),
        Err(_err) => Err(sqlx::Error::RowNotFound),
    }
}
