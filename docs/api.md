# auth API

Base path: `/api`. Auth: endpoints that carry a JWT take `Authorization:
Bearer <token>` (HS512-signed; claims `sub`, `username`, `role`, `iat`,
`exp`). Role-gated endpoints compare the `role` claim case-insensitively.
Errors: `{ "error": "message" }` for auth/rate-limit failures, and the
`AppError` shape `{ "code", "message", "details?", "timestamp" }` for
business errors (camelCase `code` values). Health is unauthenticated.

## Endpoints

### POST /api/auth/login
- **Purpose:** Sign in by username **or** email (the `username` field accepts
  both; the handler falls back to `find_by_email`). Constant response time via
  a dummy bcrypt hash when the account does not exist, to blunt username
  enumeration.
- **Auth required:** no
- **Body params:**
  | Param | Type | Required |
  |---|---|---|
  | `username` | string | yes |
  | `password` | string | yes |
- **Success 200:** `AuthResponse`
  ```json
  {
    "token": "<hs512 jwt>",
    "user": {
      "id": "64f1a2b3c4d5e6f7a8b9c0d1",
      "name": "Alice",
      "username": "alice",
      "email": "alice@example.com",
      "emailVerified": false,
      "role": "member",
      "permissions": [],
      "createdAt": "2026-08-01T09:00:00Z",
      "updatedAt": "2026-08-01T09:00:00Z"
    },
    "expiresIn": 2592000
  }
  ```
  (`permissions` is omitted when empty.)
- **Errors:**
  - 400 → `{"code":"VALIDATION_ERROR","message":"Validation failed","details":{"username":"username is required"},"timestamp":"..."}` when username/password blank
  - 400 → `{"code":"INVALID_ARGUMENT","message":"Invalid credentials","timestamp":"..."}` when credentials are wrong

### POST /api/auth/register
- **Purpose:** Create an account and immediately issue a usable local session
  JWT, then send a one-time verification link (if `EMAIL_VERIFICATION_REQUIRED`
  and mail delivery is configured). On send failure the freshly-created
  account and its verification record are rolled back so no unusable account
  is left behind.
- **Auth required:** no
- **Body params:**
  | Param | Type | Required |
  |---|---|---|
  | `username` | string | yes |
  | `email` | string | yes |
  | `password` | string | yes (min 8 chars) |
  | `name` | string | yes |
  | `role` | string | no (default `member`) |
- **Success 201:** same `AuthResponse` shape as login.
- **Errors:**
  - 400 → `VALIDATION_ERROR` with `details` for blank fields or password shorter than 8 chars
  - 400 → `INVALID_ARGUMENT` `"Registration is not yet available because the email verification service has not been configured."` when verification is required but `BREVO_API_KEY`/`MAIL_FROM_EMAIL`/public URL are missing
  - 409 → `ALREADY_EXISTS` `"Username is already taken"` / `"Email is already registered"`

### POST /api/auth/register-with-profile
- **Purpose:** Same as register but without `username`/`role` — the username is
  derived from the email's local part (`"alice@x.com"` → `"alice"`). Always
  creates with role `member`.
- **Auth required:** no
- **Body params:**
  | Param | Type | Required |
  |---|---|---|
  | `email` | string | yes |
  | `password` | string | yes (min 8 chars) |
  | `name` | string | yes |
- **Success 201:** `AuthResponse`.
- **Errors:** same as register (no username-conflict case; email conflict 409
  `"Email is already registered"`).
- **Outbound side effect:** when `SIGNUP_EVENT_URL` is configured, auth
  fire-and-forget POSTs a `user.signed_up` domain event (see below).

## Signup domain event (outbound webhook)

When the estate sets `SIGNUP_EVENT_URL` (and optionally `SIGNUP_EVENT_TOKEN`)
on the auth service's grants, auth publishes one opaque `user.signed_up`
event per successful `register` / `register-with-profile`. Emission is
fire-and-forget with a 5s timeout and never affects the register response.

- **Auth required:** outgoing call sends `Authorization: Bearer <SIGNUP_EVENT_TOKEN>` when set.
- **Request body (JSON POST):**

  ```json
  {
    "event": "user.signed_up",
    "userId": "507f1f77bcf86cd799439011",
    "username": "alice",
    "email": "alice@example.com",
    "name": "Alice",
    "role": "member",
    "at": "2026-08-17T10:00:00Z"
  }
  ```

Auth never interprets the URL or token — the composer decides who consumes
the event (e.g. point it at the notifications LXS ingest endpoint so a signup
creates an in-app notification).

### GET /api/auth/verify-email?token=…
- **Purpose:** Consume a one-time verification link. The token is
  `<recordId>.<secret>`; the email links to the frontend route
  `/auth/verify-email/?token=…`.
- **Auth required:** no
- **Query params:**
  | Param | Type | Required |
  |---|---|---|
  | `token` | string | yes |
- **Success 200:**
  ```json
  { "verified": true, "message": "Your email was verified successfully. You can now use Marketplace and negotiations." }
  ```
- **Errors:**
  - 400 → `INVALID_ARGUMENT` `"Verification token is required"` (missing `token`)
  - 400 → `INVALID_ARGUMENT` `"Invalid verification link"` (not `id.secret`)
  - 400 → `INVALID_ARGUMENT` `"This verification link is invalid or was already used"`
  - 400 → `INVALID_ARGUMENT` `"This verification link has expired. Request a new email."`

### POST /api/auth/resend-verification
- **Purpose:** Send a new verification link for the authenticated user. Marks
  all prior unused verification records as used (one active link at a time).
- **Auth required:** yes (bearer)
- **Success 202:**
  ```json
  {
    "accepted": true,
    "messageId": "<brevo message id>",
    "message": "The verification email request was accepted by Brevo. Check your inbox and spam folder."
  }
  ```
  (`messageId` may be null.) If verification is not required or already done,
  the handler short-circuits with 202 and `messageId: null`.
- **Errors:**
  - 401 → `{"error":"Unauthorized","message":"Unauthorized: missing bearer token"}` (or `invalid or expired token`)
  - 404 → `RESOURCE_NOT_FOUND` `"User not found"`

### POST /api/auth/mail
- **Purpose:** Generic transactional mail delivery. Auth resolves each
  `recipient_id` to an email and owns the Brevo credentials; the caller owns
  subject/html. Best-effort: a transient provider failure never rolls back the
  caller's state. Unknown recipients are skipped, not failed.
- **Auth required:** yes (bearer)
- **Body params:**
  | Param | Type | Required |
  |---|---|---|
  | `messages` | array | yes |
  | `messages[].recipient_id` | string (user id) | yes |
  | `messages[].subject` | string | yes |
  | `messages[].html` | string | yes |
- **Success 200:**
  ```json
  { "accepted": 1, "skipped": 0 }
  ```
- **Errors:** 401 when unauthenticated; 500 (`INTERNAL_SERVER_ERROR`) if Brevo rejects the mail (per-message failures are counted in `skipped`, not errors).

### PUT /api/auth/change-password?currentPassword=…&newPassword=…
- **Purpose:** Change the authenticated user's password. **Params are query
  strings, not a JSON body** — the legacy contract predates JSON. Target is
  always the JWT subject; no user id is accepted.
- **Auth required:** yes (bearer)
- **Query params:**
  | Param | Type | Required |
  |---|---|---|
  | `currentPassword` | string | yes |
  | `newPassword` | string | yes (min 8 chars) |
- **Success 204:** no body.
- **Errors:**
  - 400 → `VALIDATION_ERROR` for blank/weak params
  - 400 → `INVALID_ARGUMENT` `"Current password is incorrect"`
  - 401 when unauthenticated
  - 404 → `RESOURCE_NOT_FOUND` `"User not found: <id>"`

### POST /api/auth/verify-password
- **Purpose:** Verify the authenticated user's password without changing
  anything. Used for sensitive-action confirmations (e.g. deleting a member
  account).
- **Auth required:** yes (bearer)
- **Body params:**
  | Param | Type | Required |
  |---|---|---|
  | `password` | string | yes |
- **Success 200:**
  ```json
  { "valid": true }
  ```
- **Errors:** 401 when unauthenticated; 404 `RESOURCE_NOT_FOUND` `"User not found: <id>"`.

### PUT /api/auth/me
- **Purpose:** Update the authenticated user's `name` (the only identity field
  the client may edit directly). Target is always the JWT subject.
- **Auth required:** yes (bearer)
- **Body params:**
  | Param | Type | Required |
  |---|---|---|
  | `name` | string | yes (non-blank) |
- **Success 200:** `UserDto` (same shape as `user` in the login response).
- **Errors:** 400 `VALIDATION_ERROR` when `name` blank; 401 when
  unauthenticated; 404 `RESOURCE_NOT_FOUND` `"User not found: <id>"`.

### GET /api/health
- **Purpose:** Liveness probe.
- **Auth required:** no
- **Success 200:**
  ```json
  { "status": "UP" }
  ```

### GET /api/auth/verification-status
- **Purpose:** Return email-verification state for the authenticated user.
  Sibling domains must call this (not a browser flag) before allowing
  publish/negotiation/chat. When verification is not required, always reports
  `emailVerified: true`.
- **Auth required:** yes (bearer)
- **Success 200:**
  ```json
  { "emailVerified": false, "verificationExpiresInSeconds": 86400 }
  ```
- **Errors:** 401 when unauthenticated; 404 `RESOURCE_NOT_FOUND` `"User not found"`.

### GET /api/auth/access-rights
- **Purpose:** Return the access-right tokens (permissions) the authenticated
  user currently holds. Identity-derived rights (verified email → always
  `verified_user`) are combined with explicit grants (e.g. `moderator`). Auth
  reports raw tokens; it never interprets their meaning — capability mapping
  is the composition domain's rule.
- **Auth required:** yes (bearer)
- **Success 200:**
  ```json
  {
    "userId": "64f1a2b3c4d5e6f7a8b9c0d1",
    "emailVerified": true,
    "permissions": ["verified_user"]
  }
  ```
- **Errors:** 401 when unauthenticated; 404 `RESOURCE_NOT_FOUND` `"User not found"`.

### GET /api/auth/session
- **Purpose:** Return the authenticated identity for sibling domains, keeping
  JWT verification in Auth so peers never need the signing secret.
- **Auth required:** yes (bearer)
- **Success 200:** `UserDto`.
- **Errors:** 401 when unauthenticated; 404 `RESOURCE_NOT_FOUND` `"User not found"`.

### POST /api/auth/users/check-existence
- **Purpose:** Given a list of usernames, return which of them already exist
  (used for uniqueness hints during onboarding).
- **Auth required:** no
- **Body params:**
  | Param | Type | Required |
  |---|---|---|
  | `usernames` | string[] | yes (non-empty) |
- **Success 200:**
  ```json
  { "existing": ["alice", "bob"] }
  ```
- **Errors:** 400 → `VALIDATION_ERROR` with `details.usernames` =
  `"Username list cannot be empty"` for an empty array.

### GET /api/auth/users/{id}
- **Purpose:** Public identity lookup by user id. Used by peer domains to
  hydrate a profile row the first time they meet a userId and to refresh
  identity fields live on profile reads.
- **Auth required:** no
- **Path params:**
  | Param | Type | Required |
  |---|---|---|
  | `id` | string (ObjectId) | yes |
- **Success 200:** `UserDto`.
- **Errors:** 404 → `RESOURCE_NOT_FOUND` `"User not found: <id>"` (also for
  non-ObjectId ids).

### GET /api/auth/users/username/{username}
- **Purpose:** Public identity lookup by username, for peer domains to hydrate
  a `/users/username/{username}` profile view it has never seen locally.
- **Auth required:** no
- **Path params:**
  | Param | Type | Required |
  |---|---|---|
  | `username` | string | yes |
- **Success 200:** `UserDto`.
- **Errors:** 404 → `RESOURCE_NOT_FOUND` `"User not found: <username>"`.

### DELETE /api/users/{id}
- **Purpose:** Soft-delete (deactivate) a user's account (`deletedAt` set;
  the doc is not removed). **OWNER role only.**
- **Auth required:** yes (bearer, role `OWNER`)
- **Path params:**
  | Param | Type | Required |
  |---|---|---|
  | `id` | string (ObjectId) | yes |
- **Success 204:** no body.
- **Errors:** 401 unauthenticated; 403 → `ACCESS_DENIED` `"Access denied"` for
  non-OWNER roles; 404 `RESOURCE_NOT_FOUND` `"User not found: <id>"`.

> **Avatar/cover upload is not an auth endpoint.** Auth is pure identity.
> Upload avatar/cover via the `profile` domain (`POST /api/users/{id}/avatar`,
> `POST /api/users/{id}/upload-cover-photo`), which proxies the bytes to the
> `storage` LXS and stores the resulting content URL on the profile row.
> Auth no longer has `/files` routes, S3, or image processing.

## Error reference

| Code | Status | Body | When |
|---|---|---|---|
| `VALIDATION_ERROR` | 400 | `{"code":"VALIDATION_ERROR","message":"Validation failed","details":{"<field>":"<field> is required"},...}` | Blank required fields or password < 8 chars (`details` maps field → message) |
| `INVALID_ARGUMENT` | 400 | `{"code":"INVALID_ARGUMENT","message":"<reason>",...}` | Bad login credentials, bad/expired verification token, wrong current password, unconfigured email verification |
| `RESOURCE_NOT_FOUND` | 404 | `{"code":"RESOURCE_NOT_FOUND","message":"User not found: <id>",...}` | Missing user/file (also for malformed ObjectIds) |
| `ACCESS_DENIED` | 403 | `{"code":"ACCESS_DENIED","message":"Access denied",...}` | Valid JWT but role not permitted |
| `ALREADY_EXISTS` | 409 | `{"code":"ALREADY_EXISTS","message":"Username is already taken",...}` | Register with taken username/email |
| `INTERNAL_SERVER_ERROR` | 500 | `{"code":"INTERNAL_SERVER_ERROR","message":"An unexpected error occurred",...}` | Any unexpected failure (details logged server-side) |
| `Unauthorized` | 401 | `{"error":"Unauthorized","message":"Unauthorized: missing bearer token"}` / `"... invalid or expired token"` | Missing/invalid/expired `Authorization: Bearer` header |
| `rate limited` | 429 | `{"error":"Too many attempts. Please wait a moment and try again."}` | Per-source-IP token bucket exhausted |

All `AppError` bodies carry a `timestamp` (RFC3339); `details` is omitted when
empty.

## Rate limiting / limits

Per-source-IP token buckets via `SmartIpKeyExtractor` (tower-governor),
administered separately:

- **Auth-senstitive routes** (login, register, register-with-profile,
  verify-email, resend-verification, mail, change-password, verify-password,
  me): burst 5, refill 1 token / 10 s — `RATE_LIMIT_AUTH_BURST`,
  `RATE_LIMIT_AUTH_REPLENISH_SECS`.
- **General routes** (everything else): burst 120, refill 1 token / 1 s —
  `RATE_LIMIT_GENERAL_BURST`, `RATE_LIMIT_GENERAL_REPLENISH_SECS`.
  `verification-status` deliberately lives here so the frontend layout poll
  never consumes the credential-stuffing budget.
- 429 responses are JSON (see error table) with `Retry-After`/rate-limit
  headers forwarded from governor.
- **Request body cap:** 10 MB total (`RequestBodyLimitLayer`) — relevant for
  avatar/cover uploads; oversized requests are rejected with 413.
