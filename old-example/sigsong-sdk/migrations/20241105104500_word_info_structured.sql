PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS cached_word_infos;

CREATE TABLE IF NOT EXISTS word_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    search_key TEXT NOT NULL,
    word_info_id INTEGER NOT NULL,
    word TEXT NOT NULL,
    pronounce TEXT NOT NULL,
    tone TEXT NOT NULL,
    normalized_form TEXT,
    form_desc TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_word_entries_search_key ON word_entries(search_key);

CREATE TABLE IF NOT EXISTS word_senses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_id INTEGER NOT NULL REFERENCES word_entries(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    part TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS word_meanings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sense_id INTEGER NOT NULL REFERENCES word_senses(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    meaning TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS word_examples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meaning_id INTEGER NOT NULL REFERENCES word_meanings(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    example TEXT,
    translation TEXT
);

