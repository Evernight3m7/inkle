-- Rebuild FTS5 index with unicode61 tokenizer for Chinese/CJK support.
-- The default porter tokenizer doesn't handle CJK characters, making Chinese
-- text unsearchable via FTS5 MATCH.

-- Drop old FTS5 table and triggers
DROP TRIGGER IF EXISTS posts_au;
DROP TRIGGER IF EXISTS posts_ad;
DROP TRIGGER IF EXISTS posts_ai;
DROP TABLE IF EXISTS posts_fts;

-- Recreate with unicode61 tokenizer
CREATE VIRTUAL TABLE posts_fts USING fts5(
    title,
    content,
    content=posts,
    content_rowid=id,
    tokenize='unicode61'
);

-- Rebuild triggers
CREATE TRIGGER posts_ai AFTER INSERT ON posts BEGIN
    INSERT INTO posts_fts(rowid, title, content) VALUES (new.id, new.title, new.content);
END;

CREATE TRIGGER posts_ad AFTER DELETE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, content) VALUES ('delete', old.id, old.title, old.content);
END;

CREATE TRIGGER posts_au AFTER UPDATE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, content) VALUES ('delete', old.id, old.title, old.content);
    INSERT INTO posts_fts(rowid, title, content) VALUES (new.id, new.title, new.content);
END;

-- Reindex existing posts
INSERT INTO posts_fts(posts_fts) VALUES ('rebuild');
