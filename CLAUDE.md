# auth

Reusable identity and credential domain. It owns accounts, passwords, JWTs,
email ownership verification, and the identity fields `username`/`email`/
`name`/`role`. Other domains must never copy password or verification data.

Avatar/cover uploads and file storage are **not** auth's job — they belong to
the `profile` domain (which uploads to the `storage` LXS). Auth stores and
returns only credential identity; it has no storage, no S3, no image
processing, no `/files` endpoints.

## Email verification contract

`POST /api/auth/register` and `/register-with-profile` create a usable local
session, then send a verification link. The link expires after
`EMAIL_VERIFICATION_TTL_HOURS` (24 by default) and can be used once.

- `GET /api/auth/verification-status` with `Authorization: Bearer <jwt>`
  returns `{ emailVerified, verificationExpiresInSeconds }`.
- `POST /api/auth/resend-verification` with the same bearer token sends a new
  link and invalidates the previous one.
- `GET /api/auth/verify-email?token=<token>` consumes the link. Emails point
  to the frontend route `/auth/verify-email/?token=…`, which calls this API
  and shows the result in the estate's normal application layout.

Domains may allow private/local work before verification. Before a request can
publish content, start a negotiation, send public chat, or otherwise connect
one user to another, its backend must require a valid bearer token and call
`GET /auth/verification-status` at Auth. Do not trust a browser-provided
`emailVerified` flag.

## Transactional mail contract

`POST /api/auth/mail` (requires a valid bearer token) delivers fully-rendered
transactional emails on behalf of other domains:

```json
{ "messages": [ { "recipient_id": "<user id>", "subject": "…", "html": "…" } ] }
```

This contract is deliberately **content-agnostic**. Auth owns the recipient
identity (user id → email lookup) and the mail provider credentials
(`BREVO_API_KEY`, `MAIL_FROM_EMAIL`, `MAIL_FROM_NAME`); the caller owns the
message subject/html and its templates. Auth must never contain another
domain's business data or email templates, and no domain should couple to
Auth's Brevo/`MAIL_FROM_*` implementation details — use this endpoint instead.

## JWT token lifetime

Auth issues HS512-signed JWTs on login/register. The token is used by every
other domain for service-to-service and browser-to-service authentication.

- **Expiry**: controlled by `JWT_EXPIRATION` env var (milliseconds), default
  `2_592_000_000` (30 days). The login/register response includes `expires_in`
  (seconds) so the frontend can proactively invalidate the session.
- **Secret**: `JWT_SECRET` must be identical across every service in the estate
  that validates tokens (auth, chat, notifications, profile, photos, inventory,
  marketplace, bidding). Eco's `configure.sh` copies the same secret into
  every `.env`.
- **Rotation**: to rotate the secret, update `JWT_SECRET` in every service's
  `.env` and restart all services. Existing tokens become invalid immediately
  — there is no grace period.
- **Frontend behaviour**: the composition frontend stores `expires_at` derived
  from `expires_in`. Before opening/reconnecting any WebSocket (chat,
  notifications) or making authenticated requests, it checks `isSessionValid()`.
  An expired session clears localStorage and redirects to `/auth/signin/`.
  This prevents infinite 401-retry loops that previously left WebSockets
  silently dead.

## Environment and Eco

Auth uses Brevo's transactional API. Its `backend/.env.example` declares all
keys; `eco up` / `eco configure` copies missing keys into the generated `.env`
without overwriting existing secrets. Set `BREVO_API_KEY`, `MAIL_FROM_EMAIL`,
and, only when the frontend origin cannot be inferred from CORS,
`EMAIL_VERIFICATION_PUBLIC_URL` in the Eco-managed runtime `.env`, never Git.
By default Eco's first `CORS_ALLOWED_ORIGINS` value is used as the frontend
origin. Use a
verified sending domain with SPF/DKIM in Brevo.
