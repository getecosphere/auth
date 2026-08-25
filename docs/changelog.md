# auth changelog

## 3.5.0 (2026-08-25)
- Added backward-compatible multi-role identities. Auth keeps the legacy
  primary `role` and now emits the complete `roles` array in user responses
  and HS512 JWTs. Existing users with no `roles` field continue to receive
  their primary role as the full effective set.

## 3.4.1 (2026-08-23)
- Auth's composition recipe now materializes
  `services.auth-backend.config.EMAIL_VERIFICATION_REQUIRED: "true"` too, so
  the service and top-level identity policy visibly agree in ecompose.yml.

## 3.4.0 (2026-08-23)
- Email verification is now secure-by-default
  (`EMAIL_VERIFICATION_REQUIRED=true`).
- Added Auth's `compose:` recipe: `eco lxs add` can now materialize its port,
  grants, Gateway routes and visible `auth.email_verification.enabled: true`
  estate policy.
- The scoped Eco Serve relay now accepts Auth verification links as well as
  password-recovery links. Estate-owned Brevo remains the first choice.

## 3.3.1 (2026-08-23)
- Added the `darwin/arm64` artifact required by `eco up dev` / `eco serve` on
  Apple Silicon. This matches Auth's documented multi-OS local-development
  contract.

## 3.3.0 (2026-08-23)
- Password recovery can now use Eco's platform relay during an active
  `eco serve` session when an estate has not supplied its own Brevo sender.
  The relay capability is temporary and hostname-scoped; it accepts only
  Auth's reset link and is rate-limited. Estate-owned `BREVO_API_KEY` and
  `MAIL_FROM_EMAIL` remain the first-choice override.
- Added optional `EMAIL_RELAY_URL` and secret `EMAIL_RELAY_TOKEN` contract
  fields. They are Eco-managed runtime values, never manifest values.

## 3.2.0 (2026-08-22)
- Added secure password recovery: `POST /api/auth/forgot-password` always
  returns a generic 202, and `POST /api/auth/reset-password` accepts a
  bcrypt-hashed, single-use, expiring email token.
- Reset success revokes all previous sessions. Added optional
  `PASSWORD_RESET_TTL_MINUTES` (default 60, bounded to 5–1,440).
- Corrected domain ownership documentation: avatar/cover and all profile
  content belong to `profile` + `storage`, not Auth.

## 3.1.0 (2026-08-21)
- **Login rejects a second sign-in while a session is active.** Previously a
  new login silently revoked every older session ("last one wins"), which
  kicked the already-signed-in device out with no accurate explanation.
  Now `POST /api/auth/login` returns **409 `ALREADY_EXISTS`** with an accurate
  message (`"Already signed in on another device. Sign out from that device
  first."`) and the existing session is left untouched. Log out (or wait for
  the session to expire) before signing in again.
- `GET /api/auth/session-status` and the estate gateway are unchanged: a
  token whose session was revoked (logout) or expired still 401s at the edge.
- This is the intended single-session UX — the current device is the source
  of truth and stays signed in; duplicates are blocked, not displaced.

## 3.0.1 (2026-08-20)
- Artifacts: added `darwin/arm64` so estates composing `auth@3.0.0+` can run
  `eco up dev` locally (previously linux/amd64 only).

## 3.0.0 (2026-08-19)
- **Single active session per account.** Login/register mint a new session
  (`sessions` collection, one per user) that revokes every older one; the JWT
  now carries a `sid` (session id) claim. Auth's middleware and the estate
  gateway reject any token whose session is no longer active — the same
  account can no longer stay signed in on two devices.
- **Breaking:** bearer tokens without a `sid` are rejected (401) when
  `SESSION_REQUIRED` is true (default) — every client must re-login once after
  upgrade. Set `SESSION_REQUIRED=false` for a graceful legacy-token window
  (legacy tokens cannot be revoked server-side).
- **New:** `POST /api/auth/logout` revokes the caller's session;
  `GET /api/auth/session-status` reports `{ active, sessionId, expiresInSeconds,
  user }` for the presented token (the gateway polls it per protected request
  to enforce single-session at the edge).
- `AuthResponse` gains `sessionId`.
- Contract: added optional `SESSION_REQUIRED` (bool, default true); v2 `fields`
  schema updated.

## 2.1.0 (2026-08-19)
- Contract v2: `contract.env` now ships a machine-readable `fields` schema (per-key `type`, `default`, `description`, `group`, `secret`, `managed`). Same binary, same env vars, same required keys — the format gained metadata only. `required`/`optional`/`defaults` are kept as derived views so consumers predating the v2 schema (eco < 0.4.2) still resolve the contract.
- `managed:` ownership now declared in the contract: `JWT_SECRET` (shared-jwt), `MONGODB_URI` (mongo-db), `SERVER_PORT` (port), `CORS_ALLOWED_ORIGINS` (cors-origins), `ECO_AUTH_ROLES`/`ECO_AUTH_DEFAULT_ROLE` (identity-roles), `SIGNUP_EVENT_URL` (signup-event).
- Config schema spec: see `eco-server/docs/lxs-config-schema-v2.md`.

## 2.0.0 (2026-08-19)
- Logging contract: service logs now emitted as newline-delimited JSON (NDJSON) to stdout per the platform LXS logging contract (`ts`/`level`/`msg` + optional `service`,`request_id`,`status`,`latency_ms`,`user_id`,`error`). Breaking change — log output format changed.

## 1.3.0 — signup domain event (2026-08-17)

- **New:** optional `SIGNUP_EVENT_URL` (+ optional `SIGNUP_EVENT_TOKEN`). After
  each successful `register`/`register-with-profile`, auth fire-and-forget
  POSTs a `user.signed_up` event (JSON, bearer token, 5s timeout):

  ```json
  { "event": "user.signed_up", "userId": "<id>", "username": "...",
    "email": "...", "name": "...", "role": "...", "at": "<rfc3339>" }
  ```

- Auth never interprets the sink URL or token — it is a pure outbox-style
  domain event. The composer decides the consumer (e.g. the notifications LXS
  ingest endpoint). A failing or slow sink never fails registration.
- Contract: added optional `SIGNUP_EVENT_URL`, `SIGNUP_EVENT_TOKEN`; network
  outbound widened to `http, https`.

## 1.2.0 — multi-OS artifacts (2026-08-17)

- Artifacts for all five targets: linux/amd64, linux/arm64, darwin/amd64,
  darwin/arm64, windows/amd64 (same feature set as 1.1.0).

## 1.1.0 — pure identity (2026-08-16)

- **Removed** avatar/cover-photo upload, file serving, and storage (S3/MinIO)
  entirely. Auth is now pure identity: login, register, JWT, email
  verification, transactional mail, `username`/`email`/`name`/`role`.
- Avatar/cover now belong to the `profile` domain (profile proxies uploads to
  the `storage` LXS). `POST /users/:id/avatar`,
  `POST /users/:id/upload-cover-photo`, `/files/:id`, `/files/view/:id` are
  gone from auth; profile exposes `POST /users/:id/avatar` and
  `POST /users/:id/upload-cover-photo` instead.
- `avatarUrl`/`coverPhotoUrl` removed from auth's `UserDto` (profile returns
  them now).
- Dropped `image`, `webp`, `aws-sdk-s3` dependencies — linux binary shrank
  **25 MB → 10.7 MB (-57%)**, darwin 22 MB → 9.4 MB (-58%).
- Contract: removed `STORAGE_BACKEND` env; lowered resource envelope to
  memory 64m / disk 128m.

## 1.0.x — previous

Avatar upload, cover photo, file serving, S3 storage, image processing
(see 1.0.2 docs).
