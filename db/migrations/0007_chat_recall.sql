-- Rebuildable, LOCAL full-text index. Never enters the replication table list.
-- Original messages, their IDs, memories and signed event bytes stay unchanged.
CREATE VIRTUAL TABLE chat_recall_fts USING fts5(
 content, content='messages', content_rowid='rowid', tokenize='trigram'
);
INSERT INTO chat_recall_fts(rowid,content)
 SELECT rowid,content FROM messages WHERE is_deleted=0 AND status='complete' AND role IN ('user','assistant');
CREATE TRIGGER chat_recall_insert AFTER INSERT ON messages
 WHEN new.is_deleted=0 AND new.status='complete' AND new.role IN ('user','assistant') BEGIN
 INSERT INTO chat_recall_fts(rowid,content) VALUES(new.rowid,new.content);
END;
CREATE TRIGGER chat_recall_update AFTER UPDATE ON messages BEGIN
 INSERT INTO chat_recall_fts(chat_recall_fts,rowid,content)
  SELECT 'delete',old.rowid,old.content
  WHERE old.is_deleted=0 AND old.status='complete' AND old.role IN ('user','assistant');
 INSERT INTO chat_recall_fts(rowid,content)
  SELECT new.rowid,new.content
  WHERE new.is_deleted=0 AND new.status='complete' AND new.role IN ('user','assistant');
END;
CREATE TRIGGER chat_recall_delete AFTER DELETE ON messages
 WHEN old.is_deleted=0 AND old.status='complete' AND old.role IN ('user','assistant') BEGIN
 INSERT INTO chat_recall_fts(chat_recall_fts,rowid,content) VALUES('delete',old.rowid,old.content);
END;
