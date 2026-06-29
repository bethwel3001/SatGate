CREATE TABLE sites (
    id VARCHAR(255) PRIMARY KEY,
    owner_id VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    sat_balance BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE invoices (
    payment_hash VARCHAR(255) PRIMARY KEY,
    payment_request TEXT NOT NULL,
    site_id VARCHAR(255) NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    amount_sats BIGINT NOT NULL,
    created_at VARCHAR(255) NOT NULL,
    expires_at VARCHAR(255) NOT NULL
);

CREATE TABLE submissions (
    id VARCHAR(255) PRIMARY KEY,
    site_id VARCHAR(255) NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'verified',
    paid_sats BIGINT NOT NULL,
    timestamp VARCHAR(255) NOT NULL,
    payment_hash VARCHAR(255) NOT NULL UNIQUE,
    preimage VARCHAR(255) NOT NULL UNIQUE
);

CREATE INDEX idx_submissions_site_id ON submissions(site_id);
CREATE INDEX idx_sites_owner_id ON sites(owner_id);
