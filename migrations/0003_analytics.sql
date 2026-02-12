-- Analytics, images, and supporting tables

CREATE TABLE timeline_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('author', 'book', 'reading', 'genre')),
    entity_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    occurred_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    title TEXT NOT NULL,
    details_json TEXT,
    genres_json TEXT,
    reading_data_json TEXT,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL
);
CREATE INDEX idx_timeline_events_entity ON timeline_events(entity_type, entity_id);
CREATE INDEX idx_timeline_events_occurred_at ON timeline_events(occurred_at DESC);
CREATE INDEX idx_timeline_events_user_id ON timeline_events(user_id);
CREATE INDEX idx_timeline_user_occurred ON timeline_events(user_id, occurred_at DESC);

CREATE TABLE ai_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    model TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    cost REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_ai_usage_user_id ON ai_usage(user_id);

CREATE TABLE stats_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    data TEXT NOT NULL,
    computed_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id)
);
CREATE INDEX idx_stats_cache_user_id ON stats_cache(user_id);

CREATE TABLE entity_images (
    id INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('author', 'book', 'genre')),
    entity_id INTEGER NOT NULL,
    content_type TEXT NOT NULL,
    image_data BLOB NOT NULL,
    thumbnail_data BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(entity_type, entity_id)
);
CREATE INDEX idx_entity_images_lookup ON entity_images (entity_type, entity_id);

CREATE TABLE cover_suggestions (
    id TEXT PRIMARY KEY,
    image_data BLOB NOT NULL,
    thumbnail_data BLOB NOT NULL,
    content_type TEXT NOT NULL,
    source_url TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
