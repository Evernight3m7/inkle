-- Rebuild FTS5 with trigram tokenizer for universal language support (CJK + Latin)
-- Add category and tags columns to search scope
-- Add search_logs for analytics

-- Drop old FTS5 triggers and table
DROP TRIGGER IF EXISTS posts_au;
DROP TRIGGER IF EXISTS posts_ad;
DROP TRIGGER IF EXISTS posts_ai;
DROP TABLE IF EXISTS posts_fts;

-- Recreate with trigram tokenizer: handles both CJK and Latin text naturally
-- Trigram splits text into overlapping 3-char sequences, enabling substring
-- search in any language without needing a separate LIKE fallback for CJK.
CREATE VIRTUAL TABLE posts_fts USING fts5(
    title,
    content,
    category,
    tags,
    content=posts,
    content_rowid=id,
    tokenize='trigram'
);

-- Rebuild sync triggers
CREATE TRIGGER posts_ai AFTER INSERT ON posts BEGIN
    INSERT INTO posts_fts(rowid, title, content, category, tags)
    VALUES (new.id, new.title, new.content, new.category, new.tags);
END;

CREATE TRIGGER posts_ad AFTER DELETE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, content, category, tags)
    VALUES ('delete', old.id, old.title, old.content, old.category, old.tags);
END;

CREATE TRIGGER posts_au AFTER UPDATE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, content, category, tags)
    VALUES ('delete', old.id, old.title, old.content, old.category, old.tags);
    INSERT INTO posts_fts(rowid, title, content, category, tags)
    VALUES (new.id, new.title, new.content, new.category, new.tags);
END;

-- Reindex existing posts
INSERT INTO posts_fts(posts_fts) VALUES ('rebuild');

-- Search analytics table
CREATE TABLE IF NOT EXISTS search_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query TEXT NOT NULL,
    results_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
