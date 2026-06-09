CREATE TABLE IF NOT EXISTS scans (
    id TEXT PRIMARY KEY,
    target TEXT NOT NULL,
    scan_type TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'Running',
    findings_count INTEGER NOT NULL DEFAULT 0
);
