# Gotchas

Operational constraints learned in production — none of these are visible in
the binary, so read this before deploying or writing consumers.

- **JWT secret is estate-global.** `JWT_SECRET` must be byte-identical across
  every service that validates tokens (auth, chat, notifications, profile,
  photos, inventory, marketplace, bidding). Eco's `configure.sh` copies the
  same secret into every `.env`. Rotation = update every service and restart
  all of them; existing tokens become invalid immediately, **there is no grace
  period**.
- **Refuses to start with a weak/placeholder secret.** `JWT_SECRET` is
  required and must be ≥ 32 bytes (`openssl rand -base64 48`); 64 bytes is
  recommended for HS512 (below 64 logs a warning). Known placeholders
  (`your-secret-key-change-in-production`, `change-this-secret`, `secret`)
  are rejected at boot, as is any empty value.
- **JWT claim shape is a cross-service contract.** Claims are `sub`,
  `username`, `role`, `iat`, `exp`, HS512 — lms-backend's `JwtTokenProvider`
  parses exactly this shape. Changing it requires updating both services.
  Default lifetime 30 days (`JWT_EXPIRATION`, ms); responses carry
  `expiresIn` (seconds) so the frontend can proactively invalidate the
  session instead of 401-retry looping (WebSockets went silently dead before
  this was fixed).
- **Email verification is currently DISABLED until Brevo is configured.** The
  shipped `.env.example` sets `EMAIL_VERIFICATION_REQUIRED=false`. Note the
  code default is `true`: if verification is required but `BREVO_API_KEY`,
  `MAIL_FROM_EMAIL`, and the public URL are missing, `POST /api/auth/register`
  (and `register-with-profile`) return 400
  `"Registration is not yet available because the email verification service
  has not been configured."` — registration is a hard no-op on an unconfigured
  install. Set `BREVO_API_KEY` + `MAIL_FROM_EMAIL` (verified sending domain
  with SPF/DKIM) before public rollout.
- **Verification links are single-use and time-boxed.** Token format
  `<recordId>.<secret>`, link = `{public_url}/auth/verify-email/?token=…`
  with a **trailing slash** on the route (Astro emits a static
  `auth/verify-email/index.html`; without the slash a static host may 404 or
  serve the app root). Expires after `EMAIL_VERIFICATION_TTL_HOURS` (24) and
  can be used once; `resend-verification` invalidates the previous link.
- **Never trust a browser `emailVerified` flag.** Before publishing content,
  starting a negotiation, public chat, or any user↔user connection, a domain
  backend must hold a valid bearer token and call
  `GET /api/auth/verification-status`. When verification is disabled
  (`EMAIL_VERIFICATION_REQUIRED=false`) that endpoint reports
  `emailVerified: true` for everyone.
- **Register rolls back on email-send failure.** If Brevo rejects the first
  verification email, the freshly-created account and its verification record
  are deleted so no unusable account survives. Normal deactivation is a soft
  delete (`deletedAt`), and all identity lookups filter out soft-deleted rows.
- **Login timing is constant** against nonexistent users (dummy bcrypt hash),
  so response-time can't be used to enumerate usernames. Username *and* email
  both work in the `username` field.
- **`change-password` takes query params, not a JSON body**
  (`PUT /api/auth/change-password?currentPassword=…&newPassword=…`). This is a
  legacy-contract quirk; posting JSON silently yields an empty-parameter
  validation error. Target user is always the JWT subject — the old contract
  accepted an arbitrary `userId` with no auth, which is a fixed CVE-class bug.
- **Forgot-password never confirms an account exists.** `POST
  /api/auth/forgot-password` always returns 202. A real reset link is sent
  only when the address exists *and* Brevo plus the public origin are
  configured. Do not make a frontend branch on its response body.
- **Reset tokens are one-time, bcrypt-hashed, and short-lived.** The URL is
  `{origin}/reset-password?token=<recordId>.<secret>`; the secret is never
  stored plaintext. A new request invalidates a previous unused link. Reset
  success revokes every active session, so all devices must sign in again.
- **`verify-password` exists specifically so a sensitive-action confirmation
  (e.g. deleting an account) never re-posts the real password** through a
  mutating change-password call. It verifies only.
- **Rate limits are per source IP and split in two buckets.**
  Auth routes (login/register/verify-email/resend/mail/change-password/
  verify-password/forgot-password/reset-password/me): burst 5, refill 1 per 10 s. Everything else: burst
  120, refill 1 per 1 s. `verification-status` deliberately sits in the
  general bucket so the shared frontend layout poll doesn't consume the
  credential-stuffing budget. Tune via `RATE_LIMIT_AUTH_BURST` /
  `RATE_LIMIT_AUTH_REPLENISH_SECS` / `RATE_LIMIT_GENERAL_BURST` /
  `RATE_LIMIT_GENERAL_REPLENISH_SECS`. The keyed store is swept every 60 s.
- **MongoDB URI must include a database name** (e.g.
  `mongodb://localhost:27017/rwid_community`); the default database is
  `rwid_community`. Collections used: `users`, `email_verifications`,
  `password_resets`, and `sessions`. Timestamps stored via `bson::DateTime` — do not write raw
  `chrono` timestamps through `$set` updates or deserialization breaks.
- **Public identity lookups (`/auth/users/{id}`, `/auth/users/username/{u}`,
  `/health`,
  `/auth/users/check-existence`) are unauthenticated** by design — siblings
  hydrate profile rows before any user has presented a token. Malformed
  ObjectIds return 404, not 400.
- **401 vs 403 semantics.** Missing/invalid/expired bearer token → 401 with
  `{"error":"Unauthorized",...}`. A *valid* token with insufficient role →
  403 `ACCESS_DENIED`. Account deactivation requires `OWNER` only (roles
  compared case-insensitively against the JWT `role` claim).
- **Security headers are always applied** (`x-content-type-options: nosniff`,
  `x-frame-options: DENY`, `referrer-policy: no-referrer`, plus
  `Cache-Control: no-cache, no-store`). The Java predecessor sent these via
  Spring Security; the Rust rewrite re-adds them in the response middleware.
  Do not rely on the API being cacheable.
- **`x-request-id` correlation.** Reused from the incoming header when
  present, otherwise generated; echoed on the response and recorded on the
  JSON log span (filterable in Loki). Forward it to peer services for
  end-to-end tracing.
- **Transactional mail is best-effort and content-agnostic.** `POST
  /api/auth/mail` returns `{accepted, skipped}`; per-message send failures
  increment `skipped` (and unknown recipient ids are skipped), never a 500 —
  so the caller's own transaction is never rolled back by a transient Brevo
  failure. Auth owns recipient identity + provider credentials only; no
  domain should couple to Brevo/`MAIL_FROM_*` implementation details.
