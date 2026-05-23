CREATE TABLE IF NOT EXISTS post_tags (
    post_id INTEGER NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (post_id, tag),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_post_tags_tag ON post_tags(tag);

-- Migrate existing data from JSON tags column
INSERT OR IGNORE INTO post_tags (post_id, tag)
SELECT p.id, json_each.value
FROM posts p, json_each(p.tags)
WHERE p.tags IS NOT NULL AND p.tags != '[]';
