CREATE TABLE sites (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    name TEXT NOT NULL,
    sat_balance INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE invoices (
    payment_hash TEXT PRIMARY KEY,
    payment_request TEXT NOT NULL,
    site_id TEXT NOT NULL REFERENCES sites(id),
    message TEXT NOT NULL,
    amount_sats INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE submissions (
    id TEXT PRIMARY KEY,
    site_id TEXT NOT NULL REFERENCES sites(id),
    message TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'verified',
    paid_sats INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    payment_hash TEXT NOT NULL UNIQUE,
    preimage TEXT NOT NULL UNIQUE
);

CREATE INDEX idx_submissions_site_id ON submissions(site_id);
CREATE INDEX idx_sites_owner_id ON sites(owner_id);
