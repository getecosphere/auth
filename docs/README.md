# auth — LXS docs

## Capability

Identity and credentials for the whole estate. Handles registration, login
(by username or email), JWT issuance/validation (HS512), email-ownership
verification, password change/verify, the shared profile-identity fields
(name, avatar, cover photo), and transactional email delivery on behalf of
other domains. If a consumer needs to know *who* an actor is, verify a
bearer token, or send a transactional email, this is the domain.

## What it owns / never owns

- **Owns:** accounts, bcrypt password hashes, JWTs, email-ownership
  verification records, profile identity fields (`name`, `avatarUrl`,
  `coverPhotoUrl`), the `users`, `email_verifications` and `files`
  collections, Brevo mail-provider credentials.
- **Never owns:** profile content fields (bio, experiences, headline —
  owned by lms-backend now), other domains' email templates/business data,
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
