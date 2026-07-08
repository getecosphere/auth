# Auth service: Spring Boot → Rust

Branch: `rust-implementation` (this repo and `rwid/lms/backend`, kept in
lockstep). `main` is untouched and still deployable as the Spring Boot
implementation.

## Why

Auth is meant to be the lightest, most reusable domain in the estate (see
`eco/CLAUDE.md`), but the Spring Boot implementation had grown into a
general-purpose user/profile/setup/branding service — a full Spring stack
(web, security, data-mongodb, websocket, kafka) for what should be the
cheapest service to run. Rewriting it in Rust (axum) was the opportunity to
also correct the domain boundary: **auth now owns credentials and JWTs
only**, plus avatar/cover-photo upload (personal identity, not general
profile data).

## What actually moved

The original `auth/backend` was not a pure auth service. Tracing what the
frontend actually called (`client.ts`'s `isAuthRoute` routing) showed most of
it was already dead weight or a duplicate of code `lms-backend` also had and
actually served:

| Concern | Where it lived before | Where it lives now |
|---|---|---|
| Login/register/JWT/change-password | auth | **auth** (unchanged, in Rust) |
| Avatar/cover-photo upload | auth | **auth** (kept — see below) |
| Account deactivation (soft delete) | auth (`DELETE /users/{id}`) | **auth** (still gates login capability) |
| General profile (bio, headline, location, website, interests, experience, education, skills, certifications, social links) | auth (also duplicated in lms, unused) | **lms-backend** (`UserController`, newly wired to the pre-existing `UserService`) |
| Setup wizard / first-run bootstrap | auth (dead — frontend called lms's copy) | **lms-backend** (already the live implementation; only its one outbound call changed) |
| AppConfig (platform branding) | auth (dead — frontend called lms's copy) | **lms-backend** (unchanged, already correct) |
| Files (course covers, post images, etc.) | auth (dead for these types) | **lms-backend** (already correct) |
| `WebSocketConfig` / `/ws-users` | auth (registered, never wired to anything) | **deleted**. lms-backend's `UserWebSocketController` was always the real one. |
| Kafka user-event sync | auth publishes, lms consumes | **deleted**, replaced by explicit HTTP (see below) |
| `platformId` | auth (User field + JWT claim) | **lms-backend only** (never sent to auth, never in the JWT) |

### Why avatar/cover-photo stayed in auth

They look like "profile" data, but they're core account identity, not the
LinkedIn-style profile fields (experience, skills, etc.) that are genuinely
lms/RWID-specific. Keeping them in auth also sidesteps a real bug the old
split had: avatar uploads went through auth's storage while the returned URL
(`/files/view/{id}`, no service prefix) would resolve against whichever
service the frontend's routing defaulted to for `/files/*` — not necessarily
the one that stored the file. Auth's file endpoints now always return
**absolute URLs** built from `API_BASE_URL`, so this ambiguity can't recur
regardless of which service ends up owning which file type.

### Why `platformId` came out entirely

It's an LMS/RWID multi-tenancy concept ("which platform is this community/
course under"), not something a reusable auth domain should need to know
about. Evidence it didn't belong: the generic signup page always sent
`platformId: ""`, and `getPlatformIdFromToken` was dead code in both
services' JWT plumbing — nothing ever read it off an authenticated request.

## New auth contract

Base path is still `/api` (mirrors the old `server.servlet.context-path`).

- `POST /auth/login` — JSON body `{username, password}`
- `POST /auth/register` — query params `username, email, password, name, role?` (default `member`)
- `POST /auth/register-with-profile` — query params `email, password, name` (whatsappNumber/province no longer accepted here — see lms-backend notes)
- `PUT /auth/change-password` — **requires a bearer token**; query params `currentPassword, newPassword`. Target user is the authenticated principal, not a client-supplied id (see Security hardening).
- `POST /auth/users/check-existence` — JSON body `{usernames: [...]}, response {existing: [...]}`
- `GET /auth/users/{id}` — **new**, internal identity lookup for lms-backend (id/username/email/name/role/avatarUrl/coverPhotoUrl/timestamps, no passwordHash)
- `GET /auth/users/username/{username}` — **new**, same but by username
- `POST /users/{id}/avatar`, `POST /users/{id}/upload-cover-photo` — multipart, requires `OWNER`/`MENTOR`/`MEMBER` role
- `GET /files/{id}`, `GET /files/view/{id}`, `DELETE /files/{id}` — avatar/cover storage only
- `DELETE /users/{id}` — account deactivation, requires `OWNER` role
- `GET /health`

`/setup/*` and `/app-config` no longer exist on auth — they were already
served by lms-backend in practice.

## JWT compatibility

Claims are `sub`, `username`, `role`, `iat`, `exp` — `platformId` dropped.
Signing is still HS512 over the raw UTF-8 secret bytes (`jsonwebtoken`
crate's `EncodingKey::from_secret`, matching Java's
`Keys.hmacShaKeyFor(jwtSecret.getBytes())`). Verified byte-for-byte
compatible by manually recomputing the HMAC-SHA512 signature of a
Rust-issued token and confirming it matches. lms-backend's own
`JwtTokenProvider` (still Java) validates these tokens without any change to
its algorithm or secret handling — only `getPlatformIdFromToken` and the
`platformId` parameter on `generateToken` were removed since nothing used
them and the claim no longer exists.

Passwords are still bcrypt (cost 10, matching Spring's
`BCryptPasswordEncoder` default). The `bcrypt` crate verifies `$2a$`/`$2b$`
hashes interchangeably, so existing password hashes in the `users`
collection keep working without migration.

## Kafka removal

`KafkaProducerConfig`/`UserService.publishUserEvent` (auth) and
`KafkaConsumerConfig`/`UserProfileConsumer`/`UserUpdatedEvent` (lms-backend)
are gone. eco's own doctrine already treated auth-side Kafka publishing as
optional, non-composed infrastructure, and there was no near-term need for
it, so it's removed outright rather than toggled off.

Replacement is two explicit HTTP-based mechanisms lms-backend's `AuthClient`
uses, chosen instead of a shared event contract:

1. **Authorization/identity for already-authenticated requests**: read
   straight from the validated JWT (`role`, `sub`). Never stale, since it's
   re-validated on every request.
2. **Everything else** (username/email/name, and — refreshed on every
   read — avatarUrl/coverPhotoUrl): `GET /auth/users/{id}` or
   `GET /auth/users/username/{username}`, called by lms-backend's
   `UserService.syncFromAuth()`:
   - On a **profile view** (`GET /users/{id}`, `GET /users/username/{x}`),
     lms always refreshes from auth first, so avatar/cover/role changes made
     through auth show up immediately with no separate sync step.
   - On a **profile edit** (add experience, update bio, etc.), lms only
     hydrates a missing local row (so editing a profile you've never
     touched before doesn't 404) — it doesn't force a refresh on every
     write, since those don't need avatar freshness.
   - Several *other* lms services (`CommentService`, `PostService`,
     `EventService`, `SuccessStoryService`, `SlideDeckService`) read
     `avatarUrl`/`role` directly off the local `users` collection to render
     post/comment authors and permission checks. Those weren't touched —
     they keep working exactly as before, off whatever the local cache last
     had. This means avatar changes take effect there the next time that
     user's profile happens to be viewed (which re-syncs the local row),
     not instantly. Same tradeoff Kafka had (propagation delay), just pull-
     based instead of push-based, and one this rewrite deliberately accepted
     rather than building a stronger consistency mechanism nobody asked for.

## Known, deliberate behavior differences from the Java version

- **Setup no longer copies the platform's branding avatar onto the owner's
  personal avatar.** The old code set both `AppConfig.avatarUrl` and
  `User.avatarUrl` from the same setup-wizard upload. AppConfig's copy
  (platform branding, served by lms) is unaffected. The owner's personal
  avatar is simply unset until they upload one through the normal profile
  flow — auth's register endpoints don't accept an arbitrary `avatarUrl`
  string anymore, only a real upload.
- **WebP compression is lossless, not lossy Q80.** The `image` crate's pure-
  Rust WebP encoder only supports lossless encoding; lossy would require
  linking `libwebp` (a native dependency), which cuts against the point of
  this rewrite. Avatar/cover files will generally be somewhat larger than
  before. Valid, correctly-served WebP either way.
- **`/auth/register-with-profile` no longer accepts `whatsappNumber`/
  `province`.** Those are profile fields now; the frontend does a follow-up
  `PUT` to lms's `/users/{id}` (which gained `whatsappNumber`/`province`
  params for exactly this) right after registration.
- Login/register response JSON for `user` only includes fields auth
  actually owns (id, name, username, email, avatarUrl, coverPhotoUrl, role,
  timestamps) — profile fields are simply absent rather than `null`, same
  as Jackson's `NON_NULL` behavior the Java DTOs already used.

## Security hardening

Two vulnerabilities were found and fixed during this rewrite that were
**not introduced by it** — both existed in the original Java `main` branch
too and should be patched there independently of any decision to cut over
to Rust:

1. **`change-password` had no authentication at all.** `PUT
   /auth/change-password?userId=X&newPassword=Y` was `permitAll()` in
   Spring Security with no old-password check — anyone who knew or guessed
   a userId could set that account's password. Confirmed unused by the
   frontend (`git grep` found no caller), so fixed here with no
   compatibility cost: the endpoint now requires a valid bearer token, acts
   only on the authenticated principal's own account (no client-supplied
   userId), and requires `currentPassword` to verify before accepting
   `newPassword`.
2. **`register`/`register-with-profile` let anyone take over an existing
   account.** Both returned a valid session for the *existing* user when
   the submitted username/email already existed, without checking the
   submitted password matched. Anyone who knew a target's username or email
   could log in as them. Fixed by returning `409 Conflict` instead. This
   also means retrying a setup call after a network hiccup, where auth's
   registration actually succeeded but the response was lost, now surfaces
   as an error instead of silently succeeding a second time — a legitimate
   idempotency use case traded for the security fix, and one no code in
   this estate currently depends on (`SetupService` only ever calls
   register once, guarded by its own `AppConfig` existence check).

**If `main` is still deployed anywhere, both of these should be treated as
live, exploitable account-takeover bugs independent of this rewrite.**

Everything below is new hardening added specifically for the rewrite, on
top of the fixes above:

- **Timing-safe login.** The Java version (and this port, initially)
  returned immediately on "username not found," skipping the bcrypt
  comparison that the "wrong password" path pays for — response time alone
  reveals whether a username exists. Login now always runs a bcrypt verify,
  against a fixed dummy hash when the user doesn't exist, so timing is the
  same either way.
- **JWT_SECRET fails fast.** The Java (and initial Rust) config silently
  fell back to a known placeholder (`your-secret-key-change-in-production`)
  if the env var was unset. The service now refuses to start if
  `JWT_SECRET` is missing, empty, a known placeholder, or under 32 bytes
  (warns below the HS512-recommended 64).
- **Per-IP rate limiting** (`tower_governor`, keyed by
  `X-Forwarded-For`/`X-Real-IP`/`Forwarded` with a peer-IP fallback — see
  the caveat below). Login, register, register-with-profile, and
  change-password share a strict limiter (burst 5, replenishing 1 per 10s).
  Everything else gets a generous default (burst 30, replenishing 1/s).
  **This trusts forwarding headers and must only be deployed behind a
  reverse proxy that sets them correctly** (Caddy's `reverse_proxy` does by
  default) — directly internet-facing, it'd be trivially bypassable by
  spoofing the header.
- **Security response headers** restored to parity with what Spring
  Security was adding by default (confirmed via a live boot log):
  `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
  `X-XSS-Protection: 0`, `Referrer-Policy: no-referrer`, and
  `Cache-Control: no-cache, no-store, max-age=0, must-revalidate` on every
  response.
- **Request body size cap** (10MB) via `RequestBodyLimitLayer`, applied
  globally.
- **Server-side password strength** — minimum 8 characters, enforced on
  register, register-with-profile, and change-password. The frontend's own
  signup check is weaker (6) and doesn't matter for API callers who bypass
  the UI; this is now enforced where it can't be skipped.
- **Decompression-bomb guard on avatar/cover upload.** A small, highly
  compressed file can still decode to a huge pixel buffer. Image dimensions
  are now read from the format header alone (no pixel decode) and rejected
  above 30 megapixels *before* the expensive full decode + WebP re-encode
  runs.
- **Audit logging** (`tracing::warn!`, never logging credentials): failed
  logins, register conflicts, role-denied (403) actions, and account
  deactivations, each with the relevant user id/username and requested
  role where applicable.

### Not done, and why

- **Persistent/distributed account lockout** after N failed attempts was
  considered and deliberately not built. Rate limiting already covers the
  common case for a single-instance-per-estate deployment (eco's "one CT =
  one estate" model), and a lockout mechanism keyed on username is itself a
  denial-of-service vector (an attacker who knows a username can lock the
  real owner out by deliberately failing their login repeatedly). Worth
  revisiting if rate limiting proves insufficient in practice.
- **Token revocation / logout.** JWTs remain purely stateless with no
  blacklist, same as the Java version — not a regression, just not
  addressed here. A short-lived-access-token + refresh-token pattern would
  be the real fix, but is a bigger design change than this rewrite's scope.
- **TLS termination** is intentionally not in this app — matches the Java
  version (`ssl.enabled: false`) and eco's architecture, where the estate
  gateway (Caddy) and Cloudflare Tunnel own TLS.

## Running both services together locally

```
# auth (this repo)
cp .env.example .env   # set JWT_SECRET, must match lms-backend's
cargo run

# lms-backend
# .env needs AUTH_BASE_URL pointing at this service, and the same JWT_SECRET
mvn spring-boot:run
```

Verified end-to-end on this branch: setup (lms forwarding owner creation to
auth), login/register JWT round-trip, password change, avatar upload with
live composition into lms's profile view, role-gated account deactivation,
and lazy profile hydration in lms for a user it had never seen (registered
directly against auth).

Security fixes verified directly: register conflict returns 409 with no
session instead of logging in as the existing account; change-password
rejects unauthenticated requests (401) and wrong current passwords (400);
security headers present on every response; rate limiter returns 429 after
the configured burst and recovers after the replenish window; a
20000x20000-declared PNG with negligible actual pixel data is rejected
before decode instead of being processed.

## Cutover notes (not done as part of this branch)

- `rwid/rwid_bootstrap/ecompose.yml`'s `auth-backend` service declares
  `runtimes: [java@17, maven, mongodb@7]` — switching this estate over to
  the Rust implementation means changing that to a Rust/Cargo runtime, and
  is an `eco`-level decision to make when actually cutting over, not part
  of this branch.
- Both services must share `JWT_SECRET` per eco's existing shared-secret
  contract — no change to that requirement.
