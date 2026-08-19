# auth changelog

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
