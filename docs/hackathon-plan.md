# Hackathon Build Plan

## Goal

Build a working SatGo demo that proves invoice creation, payment, webhook verification, and dashboard delivery.

## MVP Scope

Build:

- Rust Axum API
- SQLite schema and migrations
- Next.js dashboard
- iframe widget
- WebLN payment path
- QR fallback
- Lightning provider webhook endpoint
- demo page with embedded widget
- concise setup documentation

Defer:

- owner withdrawals
- full account billing
- complex team management
- advanced analytics
- production PostgreSQL deployment

## Suggested Milestones

### Hours 0-8

- Scaffold Rust API and Next.js app.
- Configure pnpm for frontend package management.
- Add SQLite migrations.
- Create basic dashboard and widget routes.

### Hours 8-24

- Implement form configuration and pending message creation.
- Integrate Alby or LNBits invoice creation.
- Add invoice status polling.

### Hours 24-42

- Implement WebLN payment.
- Add QR fallback.
- Add webhook verification and replay protection.
- Mark paid messages as verified.

### Hours 42-60

- Build dashboard message view.
- Add responsive styling.
- Add status states for pending, paid, failed, and expired invoices.
- Add error and success displays using the defined color system.

### Hours 60-72

- Polish demo flow.
- Add README setup instructions.
- Add tests for webhook verification and invoice matching.
- Prepare pitch deck and demo script.

### Hours 72-84

- Freeze scope.
- Rehearse the demo.
- Fix only demo-breaking issues.
- Record a fallback demo video.

## Demo Priorities

The demo must clearly show:

- iframe embed
- invoice creation
- payment
- verified message delivery
- owner dashboard update

Everything else is secondary.
