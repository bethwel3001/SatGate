# Architecture

## Overview

SatGo has three main surfaces:

1. Dashboard: a Next.js app where site owners configure forms and view verified messages.
2. Widget: an embeddable iframe served by the frontend and backed by the Rust API.
3. API: a Rust Axum service that creates Lightning invoices, verifies provider webhooks, and stores verified messages.

## Data Flow

1. A visitor opens a page containing the SatGo iframe.
2. The visitor fills in the contact form and clicks send.
3. The widget sends the message payload to `POST /api/invoices`.
4. The Rust backend creates a pending message record and requests a Lightning invoice from Alby or LNBits.
5. The widget receives the invoice and attempts `window.webln.sendPayment`.
6. If WebLN is unavailable, the widget displays the invoice as a QR code and copyable payment request.
7. The Lightning provider sends a signed webhook to `POST /api/webhooks/lightning`.
8. The backend verifies the webhook signature and matches the paid invoice to the pending message.
9. The backend marks the message as verified and stores the payment proof.
10. The dashboard shows the verified message and updates the sats counter.

## Backend

Use Rust with:

- `axum` for HTTP routing
- `tokio` for async runtime
- `tower-http` for CORS, tracing, and rate limiting where appropriate
- `sqlx` with SQLite for compile-time checked database access
- `reqwest` for Lightning provider API calls
- `serde` and `serde_json` for request and webhook payloads
- `dotenvy` for local environment configuration

Suggested API routes:

- `POST /api/invoices`: create an invoice for a pending message
- `GET /api/invoices/:id`: check invoice and message status
- `POST /api/webhooks/lightning`: receive provider webhook events
- `GET /api/messages`: list verified messages for a form owner
- `POST /api/forms`: create or update a form configuration

## Frontend

Use Next.js App Router with:

- TypeScript
- Tailwind CSS
- pnpm
- WebLN support through a typed browser integration
- QR fallback for wallets that do not expose WebLN

Expected frontend routes:

- `/`: dashboard overview
- `/forms`: form management
- `/messages`: verified messages
- `/embed/[formId]`: iframe widget
- `/demo`: demo site for the hackathon presentation

## Database

SQLite is the MVP database. Keep schema design compatible with PostgreSQL by avoiding SQLite-only shortcuts where possible.

Core tables:

- `forms`: owner form configuration
- `messages`: submitted message content and verification status
- `invoices`: Lightning invoice metadata, amount, status, and payment proof
- `webhook_events`: raw provider event records for replay protection and debugging

## Security

- Verify Lightning provider webhook signatures.
- Store and validate invoice identifiers before accepting a paid event.
- Reject duplicate webhook events.
- Rate limit invoice creation by IP, form ID, and time window.
- Validate widget origins against configured domains.
- Do not expose provider API keys to the frontend.
- Keep paid and unpaid messages separate in queries.

## Production Path

After the hackathon:

- Move from SQLite to PostgreSQL.
- Add authenticated owner accounts.
- Add configurable fees and withdrawal support.
- Add sponsor mode for accessibility.
- Add analytics for conversion, payments, and blocked spam.
