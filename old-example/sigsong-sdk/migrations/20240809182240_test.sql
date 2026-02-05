CREATE TABLE IF NOT EXISTS cached_songs (
    id INTEGER PRIMARY KEY,
    payload BLOB NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cached_word_infos (
    search_key TEXT PRIMARY KEY,
    payload BLOB NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS cached_song_recommendations (
    cache_key TEXT PRIMARY KEY,
    payload BLOB NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
