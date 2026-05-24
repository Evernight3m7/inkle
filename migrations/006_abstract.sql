-- Add abstract column to posts
ALTER TABLE posts ADD COLUMN abstract TEXT DEFAULT '';

-- Rebuild FTS5 to include abstract column
DROP TRIGGER IF EXISTS posts_au;
DROP TRIGGER IF EXISTS posts_ad;
DROP TRIGGER IF EXISTS posts_ai;
DROP TABLE IF EXISTS posts_fts;

CREATE VIRTUAL TABLE posts_fts USING fts5(
    title,
    content,
    abstract,
    category,
    tags,
    content=posts,
    content_rowid=id,
    tokenize='trigram'
);

CREATE TRIGGER posts_ai AFTER INSERT ON posts BEGIN
    INSERT INTO posts_fts(rowid, title, content, abstract, category, tags)
    VALUES (new.id, new.title, new.content, new.abstract, new.category, new.tags);
END;

CREATE TRIGGER posts_ad AFTER DELETE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, content, abstract, category, tags)
    VALUES ('delete', old.id, old.title, old.content, old.abstract, old.category, old.tags);
END;

CREATE TRIGGER posts_au AFTER UPDATE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, content, abstract, category, tags)
    VALUES ('delete', old.id, old.title, old.content, old.abstract, old.category, old.tags);
    INSERT INTO posts_fts(rowid, title, content, abstract, category, tags)
    VALUES (new.id, new.title, new.content, new.abstract, new.category, new.tags);
END;

INSERT INTO posts_fts(posts_fts) VALUES ('rebuild');
