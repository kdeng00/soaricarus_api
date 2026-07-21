use sqlx::Row;

pub async fn insert(
    pool: &sqlx::PgPool,
    song: &simodels::song::Song,
) -> Result<(time::OffsetDateTime, uuid::Uuid), sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO "song" (title, artist, album_artist, album, genre, year, track, disc, track_count, disc_count, duration, audio_type, filename, directory, user_id) 
        VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) RETURNING date_created, id;
        "#
        )
        .bind(&song.title)
        .bind(&song.artist)
        .bind(&song.album_artist)
        .bind(&song.album)
        .bind(&song.genre)
        .bind(song.year)
        .bind(song.track)
        .bind(song.disc)
        .bind(song.track_count)
        .bind(song.disc_count)
        .bind(song.duration)
        .bind(&song.audio_type)
        .bind(&song.filename)
        .bind(&song.directory)
        .bind(song.user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            eprintln!("Error inserting query: {e}");
        });

    match result {
        Ok(row) => {
            let id: uuid::Uuid = row
                .try_get("id")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap();
            let date_created_time: time::OffsetDateTime = row
                .try_get("date_created")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap();
            let date_created = date_created_time;

            Ok((date_created, id))
        }
        Err(_) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn get_song(
    pool: &sqlx::PgPool,
    id: &uuid::Uuid,
) -> Result<simodels::song::Song, sqlx::Error> {
    let result = sqlx::query(
        r#"
        SELECT * FROM "song" WHERE id = $1
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
            let date_created_time: time::OffsetDateTime = row
                .try_get("date_created")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap();

            Ok(simodels::song::Song {
                id: row
                    .try_get("id")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                title: row
                    .try_get("title")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                artist: row
                    .try_get("artist")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                album_artist: row
                    .try_get("album_artist")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                album: row
                    .try_get("album")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                genre: row
                    .try_get("genre")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                year: row
                    .try_get("year")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                track: row
                    .try_get("track")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                disc: row
                    .try_get("disc")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                track_count: row
                    .try_get("track_count")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                disc_count: row
                    .try_get("disc_count")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                duration: row
                    .try_get("duration")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                audio_type: row
                    .try_get("audio_type")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                filename: row
                    .try_get("filename")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                directory: row
                    .try_get("directory")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                date_created: Some(date_created_time),
                user_id: row
                    .try_get("user_id")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                data: Vec::new(),
            })
        }
        Err(_) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn get_all_songs(pool: &sqlx::PgPool) -> Result<Vec<simodels::song::Song>, sqlx::Error> {
    let result = sqlx::query(
        r#"
        SELECT * FROM "song";
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        eprintln!("Error querying data: {e:?}");
    });

    match result {
        Ok(rows) => {
            let mut songs: Vec<simodels::song::Song> = Vec::new();

            for row in rows {
                let date_created_time: time::OffsetDateTime = row
                    .try_get("date_created")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap();

                let song = simodels::song::Song {
                    id: row
                        .try_get("id")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    title: row
                        .try_get("title")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    artist: row
                        .try_get("artist")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    album_artist: row
                        .try_get("album_artist")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    album: row
                        .try_get("album")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    genre: row
                        .try_get("genre")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    year: row
                        .try_get("year")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    track: row
                        .try_get("track")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    disc: row
                        .try_get("disc")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    track_count: row
                        .try_get("track_count")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    disc_count: row
                        .try_get("disc_count")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    duration: row
                        .try_get("duration")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    audio_type: row
                        .try_get("audio_type")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    filename: row
                        .try_get("filename")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    directory: row
                        .try_get("directory")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    date_created: Some(date_created_time),
                    user_id: row
                        .try_get("user_id")
                        .map_err(|_e| sqlx::Error::RowNotFound)
                        .unwrap(),
                    data: Vec::new(),
                };

                songs.push(song);
            }

            Ok(songs)
        }
        Err(_err) => Err(sqlx::Error::RowNotFound),
    }
}

pub async fn delete_song(
    pool: &sqlx::PgPool,
    id: &uuid::Uuid,
) -> Result<simodels::song::Song, sqlx::Error> {
    let result = sqlx::query(
        // simodels::song::Song,
        r#"
        DELETE FROM "song"
        WHERE id = $1
        RETURNING id, title, artist, album, album_artist, genre, year, disc, track, track_count, disc_count, duration, audio_type, date_created, filename, directory, user_id
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        eprintln!("Error deleting data: {e:?}")
    });

    match result {
        Ok(row) => {
            let date_created_time: time::OffsetDateTime = row
                .try_get("date_created")
                .map_err(|_e| sqlx::Error::RowNotFound)
                .unwrap();

            Ok(simodels::song::Song {
                id: row
                    .try_get("id")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                title: row
                    .try_get("title")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                artist: row
                    .try_get("artist")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                album_artist: row
                    .try_get("album_artist")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                album: row
                    .try_get("album")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                genre: row
                    .try_get("genre")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                year: row
                    .try_get("year")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                track: row
                    .try_get("track")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                disc: row
                    .try_get("disc")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                track_count: row
                    .try_get("track_count")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                disc_count: row
                    .try_get("disc_count")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                duration: row
                    .try_get("duration")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                audio_type: row
                    .try_get("audio_type")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                filename: row
                    .try_get("filename")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                directory: row
                    .try_get("directory")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                date_created: Some(date_created_time),
                user_id: row
                    .try_get("user_id")
                    .map_err(|_e| sqlx::Error::RowNotFound)
                    .unwrap(),
                data: Vec::new(),
            })
        }
        Err(_) => Err(sqlx::Error::RowNotFound),
    }
}
