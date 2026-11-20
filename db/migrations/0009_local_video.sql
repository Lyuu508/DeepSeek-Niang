CREATE TABLE video_local_media (
 source_id TEXT PRIMARY KEY REFERENCES video_sources(id),
 device_id TEXT NOT NULL REFERENCES devices(id),
 local_path TEXT NOT NULL,
 container TEXT NOT NULL CHECK(container IN ('mov','matroska')),
 byte_length INTEGER NOT NULL CHECK(byte_length BETWEEN 1 AND 268435456),
 has_audio INTEGER NOT NULL CHECK(has_audio IN (0,1)),
 imported_at_ms INTEGER NOT NULL CHECK(imported_at_ms >= 0)
);
CREATE INDEX video_local_budget ON video_local_media(device_id,imported_at_ms);
CREATE INDEX video_reusable_segments ON video_segments(media_checksum,start_ms,end_ms,status);

-- Reinstalled by install_capture with the local-video predicate.
DROP TRIGGER IF EXISTS lan_task_queue_insert;
DROP TRIGGER IF EXISTS lan_task_queue_update;
DROP TRIGGER IF EXISTS lan_task_queue_delete;
DROP TRIGGER IF EXISTS lan_task_checkpoints_insert;
DROP TRIGGER IF EXISTS lan_task_checkpoints_update;
DROP TRIGGER IF EXISTS lan_task_checkpoints_delete;
