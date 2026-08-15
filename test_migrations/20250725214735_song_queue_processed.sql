-- Add migration script here
INSERT INTO "song" (id, title, artist, album_artist, album, genre, year, track, disc, track_count, disc_count, duration, audio_type, date_created, filename, directory, file_key, user_id) 
VALUES('44cf7940-34ff-489f-9124-d0ec90a55af9', 'Hypocrite Like The Rest', 'Kuoth', 'Kuoth', 'I', 'Alternative Hip-Hop', 2020, 1, 1, 9, 1, 139, 'flac', '2020-01-01 13:00:00-05', 'track01.flac', 'tests/I', 'processed/song/track01.flac', '47491f9b-725a-4ba4-b9a5-711e1be46670');

INSERT INTO "coverart" (id, title, directory, filename, file_type, file_key, song_id) 
VALUES('996122cd-5ae9-4013-9934-60768d3006ed', 'I', 'tests/I', 'Coverart-1.jpg', 'jpeg', 'processed/coverart/Coverart-1.jpg', '44cf7940-34ff-489f-9124-d0ec90a55af9');

INSERT INTO "song" (id, title, artist, album_artist, album, genre, year, track, disc, track_count, disc_count, duration, audio_type, date_created, filename, directory, file_key, user_id) 
VALUES('94cf7940-34ff-489f-9124-d0ec90a55af4', 'It''s Too Late', 'Kuoth', 'Kuoth', 'I', 'Alternative Hip-Hop', 2020, 2, 1, 9, 1, 116, 'flac', '2020-01-01 13:01:00-05', 'track02.flac', 'tests/I', 'processed/song/track02.flac', '47491f9b-725a-4ba4-b9a5-711e1be46670');
INSERT INTO "coverart" (id, title, directory, filename, file_type, file_key, song_id) 
VALUES('d96122cd-5ae9-4013-9934-60768d3006e9', 'I', 'tests/I', 'Coverart-2.jpg', 'jpeg', 'processed/coverart/Coverart-2.jpg', '94cf7940-34ff-489f-9124-d0ec90a55af4');
