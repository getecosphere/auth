# auth — LXS docs

## Capability

Identity and credentials for the whole estate. Handles registration, login
(by username or email), JWT issuance/validation (HS512), email-ownership
verification, password recovery, password change/verify, and transactional email delivery on behalf of
other domains. If a consumer needs to know *who* an actor is, verify a
bearer token, or send a transactional email, this is the domain.

## What it owns / never owns

- **Owns:** accounts, bcrypt password hashes, JWTs, email-ownership
  verification records, password-reset records, the `users`,
  `email_verifications`, `password_resets`, and `sessions` collections, and
  password-recovery delivery policy. An estate may own its Brevo credentials;
  an active `eco serve` lease may instead use Eco's tightly scoped recovery
  relay without receiving a provider key.
- **Never owns:** profile content fields or avatars (bio, avatar, headline,
  and other person-facing data belong to `profile` + `storage`), other
  domains' email templates/business data,
  the rules that map access-right tokens to capabilities (that lives in the
  composition/core domain).

## Compose it

```yaml
# ecompose.yml
services:
  auth-backend:
    lxs: auth@1.0.2
    grants:
      secrets: [JWT_SECRET, MONGODB_URI, SERVER_PORT]
```

## Quick usage

```bash
# health
curl -s http://localhost:8080/api/health
# {"status":"UP"}

# login (username or email both work in the `username` field)
curl -s -X POST http://localhost:8080/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"s3cret-pass"}'
# 200 -> {"token":"<jwt>","user":{...},"expiresIn":2592000}

# use the token
curl -s http://localhost:8080/api/auth/session \
  -H "Authorization: Bearer <jwt>"
# 200 -> {"id":"...","name":"Alice","username":"alice","email":"...","emailVerified":false,"role":"member","permissions":[],"createdAt":"...","updatedAt":"..."}
```

## Password recovery delivery

`POST /api/auth/forgot-password` always returns the same 202 response. For a
known account, Auth prefers estate-owned `BREVO_API_KEY` + `MAIL_FROM_EMAIL`.
When those are absent during `eco serve`, Eco injects a temporary
`EMAIL_RELAY_URL` + `EMAIL_RELAY_TOKEN`: it can deliver only Auth's reset
template to the live Serve hostname and expires with that lease. It is not a
generic outbound-email service and must never be copied into an estate's
manifest or permanent environment. If neither route is available, Auth keeps
the generic 202 response and creates no recovery token.

## Docs index

- `api.md` — full endpoint reference with request/response JSON and errors
- `examples.sh` — executable smoke test (golden request→response pairs)
- `openapi.json` — machine-readable OpenAPI 3.0 spec
- `changelog.md` — version history + breaking changes
- `gotchas.md` — production-learned constraints and operational gotchas

## For AI agents

This LXS is distributed as a **binary only** — these docs are the entire
interface. Match `api.md` shapes exactly; run `examples.sh` against a pulled
binary or live estate URL before trusting behavior. See
`docs/gotchas.md` for constraints that are invisible in the binary.
