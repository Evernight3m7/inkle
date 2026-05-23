CREATE TABLE posts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT NOT NULL,
    slug        TEXT NOT NULL UNIQUE,
    category    TEXT DEFAULT '',
    tags        TEXT DEFAULT '[]',
    content     TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'draft',
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE configs (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO configs (key, value) VALUES ('site_title',      'My Blog');
INSERT INTO configs (key, value) VALUES ('site_subtitle',   '');
INSERT INTO configs (key, value) VALUES ('active_theme',    'default');
INSERT INTO configs (key, value) VALUES ('posts_per_page',  '10');
INSERT INTO configs (key, value) VALUES ('social_links',    '{}');
