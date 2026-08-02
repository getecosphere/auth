# auth

Reusable identity and credential domain. It owns accounts, passwords, JWTs,
email ownership verification, and profile-photo identity fields. Other domains
must never copy password or verification data.

## Email verification contract

`POST /api/auth/register` and `/register-with-profile` create a usable local
session, then send a verification link. The link expires after
`EMAIL_VERIFICATION_TTL_HOURS` (24 by default) and can be used once.

- `GET /api/auth/verification-status` with `Authorization: Bearer <jwt>`
  returns `{ emailVerified, verificationExpiresInSeconds }`.
- `POST /api/auth/resend-verification` with the same bearer token sends a new
  link and invalidates the previous one.
- `GET /api/auth/verify-email?token=<token>` consumes the link.

Domains may allow private/local work before verification. Before a request can
publish content, start a negotiation, send public chat, or otherwise connect
one user to another, its backend must require a valid bearer token and call
`GET /auth/verification-status` at Auth. Do not trust a browser-provided
`emailVerified` flag.

## Environment and Eco

Auth uses Brevo's transactional API. Its `backend/.env.example` declares all
keys; `eco up` / `eco configure` copies missing keys into the generated `.env`
without overwriting existing secrets. Set `BREVO_API_KEY`, `MAIL_FROM_EMAIL`,
and `AUTH_PUBLIC_URL` in the Eco-managed runtime `.env`, never Git. Use a
verified sending domain with SPF/DKIM in Brevo.
