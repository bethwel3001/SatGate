# SatGo Pitch

## One-Liner

SatGo is an embeddable contact form that replaces CAPTCHA friction with a tiny Lightning payment.

## Short Pitch

Website contact forms are broken. CAPTCHAs annoy real users, hurt conversion, and are increasingly bypassed by automated systems. SatGo solves the problem with economic proof of intent: before a message reaches the site owner, the sender pays a tiny amount, usually 1 to 10 sats.

For a real visitor with a Lightning wallet, payment can be one click through WebLN. For a spam operation, sending thousands of messages now has a direct financial cost. Website owners get cleaner inboxes, higher-intent messages, and small revenue instead of another anti-spam bill.

## Important Claim Adjustments

Use precise claims during judging:

- Say "raises the cost of spam" instead of "makes spam impossible."
- Say "high-intent paid messages" instead of "100% human messages."
- Say "reduces CAPTCHA friction" instead of "frictionless for everyone."
- Say "QR fallback supports mobile wallets" to address visitors without WebLN.

These versions are stronger because they are easier to defend in Q&A.

## Three-Minute Pitch Outline

### Slide 1: Hook

Every day, spam bots waste site owners' time and bury real leads. We have tried fighting bots with puzzles, but AI keeps getting better at solving puzzles. SatGo fights spam with economics instead.

### Slide 2: Problem

Show a contact form flooded with spam beside a visitor abandoning a CAPTCHA. The current system punishes real users while still letting spam through.

### Slide 3: Solution

SatGo is an embeddable contact form that requires a tiny Lightning payment before a message is delivered. A real visitor pays a few sats. A spammer trying to hit thousands of forms faces a real cost.

### Slide 4: Live Demo

Demo flow:

1. Show the dashboard.
2. Open a demo website with the SatGo iframe.
3. Submit a message.
4. Pay through WebLN or scan the QR code.
5. Show the success state.
6. Return to the dashboard and show the verified message plus sats counter.

### Slide 5: Architecture

The backend is Rust with Axum, Tokio, and SQLx. The frontend is Next.js, TypeScript, and Tailwind CSS. Lightning invoice generation and webhook delivery come from Alby or LNBits, so the MVP avoids node-management overhead.

### Slide 6: Business Model

SatGo can charge a tiny routing fee on paid submissions. Post-hackathon, it can expand into comment protection, API endpoint protection, sponsor mode, and richer analytics for website owners.

## Q&A Prep

Likely judge questions:

- What about people without Lightning wallets?
- How do you prevent fake webhook confirmations?
- What stops a spammer with money?
- Why is this better than CAPTCHA?
- How do website owners install it?

Suggested answers:

- WebLN is the fastest path, QR is the fallback, and sponsor mode is planned for accessibility.
- Webhook signatures, invoice matching, preimage checks, and replay protection prevent spoofed confirmations.
- SatGo does not make spam impossible; it changes spam from free to paid, which destroys the economics of bulk abuse.
- CAPTCHA adds cognitive friction to humans. SatGo adds economic friction to abuse.
- Owners paste a generated iframe snippet into their site.
