-- value is JSON
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
-- data is the serde JSON of kiri_core::model::Job
CREATE TABLE jobs (id INTEGER PRIMARY KEY, data TEXT NOT NULL);
-- holds next_job_id
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
