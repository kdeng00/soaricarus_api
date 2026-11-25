-- Add migration script here
CREATE TABLE IF NOT EXISTS "playbackQueue" (
	id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
	name TEXT NOT NULL,
	device_type TEXT CHECK (device_type IN ('BROWSER', 'MOBILE')),
	user_id UUID NOT NULL,
	date_created timestamptz DEFAULT now()
);

CREATE TABLE IF NOT EXISTS "playbackQueueItem" (
	id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
	queue_index INT NOT NULL,
	song_id UUID NOT NULL,
	playback_queue_id UUID NOT NULL,
	date_created timestamptz DEFAULT now()
);
