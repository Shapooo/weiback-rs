-- Enable FTS5 by creating the virtual table with trigram tokenizer
CREATE VIRTUAL TABLE posts_fts USING fts5(
    text,
    content='posts',
    content_rowid='id',
    tokenize='trigram'
);

-- Populate the FTS index with existing data
INSERT INTO posts_fts(rowid, text)
SELECT id, text FROM posts;

-- Triggers to keep the FTS index in sync with the posts table

-- 1. After Insert: Add new post to FTS index
CREATE TRIGGER posts_ai AFTER INSERT ON posts BEGIN
  INSERT INTO posts_fts(rowid, text) VALUES (new.id, new.text);
END;

-- 2. After Delete: Remove deleted post from FTS index
CREATE TRIGGER posts_ad AFTER DELETE ON posts BEGIN
  INSERT INTO posts_fts(posts_fts, rowid, text) VALUES('delete', old.id, old.text);
END;

-- 3. After Update: Update FTS index when text changes
CREATE TRIGGER posts_au AFTER UPDATE OF text ON posts BEGIN
  INSERT INTO posts_fts(posts_fts, rowid, text) VALUES('delete', old.id, old.text);
  INSERT INTO posts_fts(rowid, text) VALUES (new.id, new.text);
END;
