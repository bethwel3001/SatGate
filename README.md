# SatGate

SatGate is an embeddable Lightning-powered contact form that uses a tiny satoshi payment as economic proof of intent.

## Tagline

Economic Proof of Humanity. The 1-Sat Anti-Spam Contact Form.

## Problem

Website contact forms are flooded with spam. CAPTCHAs add friction for real users, hurt conversion, and are increasingly solvable by automated systems. Site owners often pay for filtering tools while still losing time and missing real leads.

## Solution

SatGate replaces puzzle-based proof with a small Lightning payment. A visitor submits a message, pays a configurable amount such as 1 to 10 sats, and the message is delivered only after the backend verifies the payment.

For visitors, WebLN enables one-click payment when a compatible wallet is installed, with a QR fallback for mobile wallets. For website owners, the widget reduces spam while creating small revenue from high-intent messages.

## Hackathon MVP

The MVP should prove the full loop:

1. Website owner creates a SatGate form in the dashboard.
2. Owner embeds the generated iframe on a demo site.
3. Visitor writes a message and requests a Lightning invoice.
4. Visitor pays through WebLN or QR fallback.
5. Backend verifies the Lightning provider webhook.
6. Verified message appears in the dashboard with a sats counter.

## Current MVP

This repository now includes a working local MVP scaffold:

- `services/api`: Rust Axum API with SQLite-backed forms, messages, invoices, webhook events, and a demo payment endpoint
- `apps/web`: Next.js dashboard, demo website, and embeddable iframe widget
- `/`: marketing landing page
- `/dashboard`: owner dashboard
- `/demo`: pitch-ready demo website
- `/embed/demo-form`: iframe widget route seeded by the API

The MVP supports WebLN attempts and QR display. Because real Alby or LNBits credentials are not configured yet, the widget also includes a demo payment action that marks the generated invoice paid through the backend.

## Local Setup

Install frontend dependencies:

```bash
pnpm install
```

Run the Rust API:

```bash
pnpm api:dev
```

In a second terminal, run the frontend:

```bash
pnpm dev
```

Open:

- Landing page: `http://localhost:3000`
- Dashboard: `http://localhost:3000/dashboard`
- Demo site: `http://localhost:3000/demo`
- API health: `http://localhost:8080/health`

Copy environment examples if needed:

```bash
cp services/api/.env.example services/api/.env
cp apps/web/.env.example apps/web/.env.local
```

## Tech Stack

- Backend: Rust, Axum, Tokio, SQLx
- Frontend: Next.js App Router, TypeScript, Tailwind CSS
- Package manager: pnpm for frontend workspaces and scripts
- Database: SQLite for the hackathon MVP, with a clear PostgreSQL migration path
- Lightning provider: Alby API or LNBits
- Wallet integration: WebLN with QR fallback

## Demo Payment Flow

For hackathon reliability, the first MVP has a mock settlement endpoint:

```text
POST /api/invoices/:id/mock-pay
```

Real provider support should replace only the invoice creation and webhook verification internals. The frontend contract can stay the same.

## Design Direction

- Fonts: Comfortaa variants for brand and display text, Calibri or a metrically compatible fallback for body text
- Colors: black, white, blue, red or yellow for errors and warnings, green for success
- Layout: fully responsive from mobile to desktop
- Tone: clean, practical, trustworthy, and fast

Do not hardcode emojis in source code. Avoid unnecessary comments; use comments only where they clarify non-obvious logic.

## Documentation

- [Architecture](docs/architecture.md)
- [Pitch](docs/pitch.md)
- [Hackathon Build Plan](docs/hackathon-plan.md)
- [Design System](docs/design-system.md)
