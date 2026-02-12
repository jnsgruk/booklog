-- Book tracking tables

CREATE TABLE genres (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE UNIQUE INDEX idx_genres_name ON genres(LOWER(TRIM(name)));

CREATE TABLE authors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE UNIQUE INDEX idx_authors_name ON authors(LOWER(TRIM(name)));

CREATE TABLE books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    isbn TEXT,
    description TEXT,
    page_count INTEGER,
    year_published INTEGER,
    publisher TEXT,
    language TEXT,
    primary_genre_id INTEGER REFERENCES genres(id) ON DELETE SET NULL,
    secondary_genre_id INTEGER REFERENCES genres(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE UNIQUE INDEX idx_books_title ON books(LOWER(TRIM(title)));
CREATE INDEX idx_books_isbn ON books(isbn) WHERE isbn IS NOT NULL;
CREATE INDEX idx_books_primary_genre ON books(primary_genre_id);
CREATE INDEX idx_books_secondary_genre ON books(secondary_genre_id);

CREATE TABLE book_authors (
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    author_id INTEGER NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'author' CHECK (role IN ('author', 'editor', 'translator')),
    PRIMARY KEY (book_id, author_id, role)
);
CREATE INDEX idx_book_authors_author ON book_authors(author_id);

CREATE TABLE user_books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    shelf TEXT NOT NULL DEFAULT 'library' CHECK (shelf IN ('library', 'wishlist')),
    book_club INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(user_id, book_id)
);
CREATE INDEX idx_user_books_user_id ON user_books(user_id);
CREATE INDEX idx_user_books_book_id ON user_books(book_id);
CREATE INDEX idx_user_books_shelf ON user_books(user_id, shelf);

CREATE TABLE readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'reading' CHECK (status IN ('reading', 'read', 'abandoned')),
    started_at TEXT,
    finished_at TEXT,
    rating REAL CHECK (rating IS NULL OR (rating >= 0.5 AND rating <= 5.0 AND (rating * 2) = CAST(rating * 2 AS INTEGER))),
    review TEXT,
    format TEXT CHECK (format IS NULL OR format IN ('physical', 'ereader', 'audiobook')),
    owned INTEGER CHECK (owned IS NULL OR owned IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_readings_book_id ON readings(book_id);
CREATE INDEX idx_readings_status ON readings(status);
CREATE INDEX idx_readings_user_id ON readings(user_id);
CREATE INDEX idx_readings_user_status_finished ON readings(user_id, status, finished_at);
