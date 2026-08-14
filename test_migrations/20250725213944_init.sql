-- Add migration script here
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Table to store queued songs to process
CREATE TABLE IF NOT EXISTS "songQueue" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    filename TEXT NOT NULL,
    status TEXT CHECK (status IN ('pending', 'ready', 'processing', 'done')),
    user_id UUID NULL
);

CREATE TABLE IF NOT EXISTS "songQueueData" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_key TEXT NOT NULL,
    bucket TEXT NOT NULL,
    region TEXT NOT NULL,
    song_queue_id UUID NOT NULL
);

-- Table to store queued metadata
CREATE TABLE IF NOT EXISTS "metadataQueue" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metadata jsonb NOT NULL,
    created_at timestamptz DEFAULT now(),
    song_queue_id UUID NOT NULL
);

-- Table to store queued coverart
CREATE TABLE IF NOT EXISTS "coverartQueue" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_type TEXT NOT NULL,
    song_queue_id UUID NULL
);

CREATE TABLE IF NOT EXISTS "coverartQueueData" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_key TEXT NOT NULL,
    bucket TEXT NOT NULL,
    region TEXT NOT NULL,
    coverart_queue_id UUID NOT NULL
);

-- Create an index for better query performance
CREATE INDEX metadata_queue_data_metadata ON "metadataQueue" USING gin (metadata);

-- Table to store a song's info
CREATE TABLE IF NOT EXISTS "song" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album_artist TEXT NOT NULL,
    album TEXT NOT NULL,
    genre TEXT NOT NULL,
    year INT NOT NULL,
    track INT NOT NULL,
    disc INT NOT NULL,
    track_count INT NOT NULL,
    disc_count INT NOT NULL,
    duration INT NOT NULL,
    audio_type TEXT NOT NULL,
    date_created timestamptz DEFAULT now(),
    filename TEXT NOT NULL,
    directory TEXT NOT NULL,
    user_id UUID NULL
);


CREATE TABLE IF NOT EXISTS "coverart" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    directory TEXT NOT NULL,
    filename TEXT NOT NULL,
    file_type TEXT NOT NULL,
    song_id UUID NOT NULL
);
