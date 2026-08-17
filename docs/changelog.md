# auth changelog

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
