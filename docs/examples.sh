#!/usr/bin/env bash
# auth LXS smoke test — golden request→response pairs.
# Usage: BASE_URL=<http://host:port> ./examples.sh
# Runs against a pulled binary or a live estate URL; every curl must succeed
# and return the documented shape or the script exits non-zero.
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
API="$BASE_URL/api"
TMP="${TMPDIR:-/tmp}/auth-lxs-smoke"

mkdir -p "$TMP"

# 1) health
code=$(curl -s -o "$TMP/health.out" -w '%{http_code}' "$API/health")
test "$code" = "200"
rg -q '"status"\s*:\s*"UP"' "$TMP/health.out"
echo "OK health -> 200"

# 2) login with bad credentials -> 400 Invalid credentials
code=$(curl -s -o "$TMP/login.out" -w '%{http_code}' -X POST "$API/auth/login" \
  -H 'Content-Type: application/json' \
  -d '{"username":"definitely-not-a-user","password":"wrong-password"}')
test "$code" = "400"
rg -q '"Invalid credentials"' "$TMP/login.out"
echo "OK login (bad creds) -> 400"

# 3) register with blank fields -> 400 VALIDATION_ERROR (never reaches Brevo)
code=$(curl -s -o "$TMP/register.out" -w '%{http_code}' -X POST "$API/auth/register" \
  -H 'Content-Type: application/json' \
  -d '{"username":"","email":"","password":"x","name":""}')
test "$code" = "400"
rg -q 'VALIDATION_ERROR' "$TMP/register.out"
echo "OK register (blank fields) -> 400"

# 4) check-existence with a non-empty list -> 200
code=$(curl -s -o "$TMP/check.out" -w '%{http_code}' -X POST "$API/auth/users/check-existence" \
  -H 'Content-Type: application/json' \
  -d '{"usernames":["no-such-user-xyz"]}')
test "$code" = "200"
rg -q '"existing"\s*:\s*\[\]' "$TMP/check.out"
echo "OK check-existence -> 200"

# 5) identity lookup for a malformed id -> 404 (treated as not found)
code=$(curl -s -o "$TMP/user.out" -w '%{http_code}' "$API/auth/users/not-an-objectid")
test "$code" = "404"
rg -q 'RESOURCE_NOT_FOUND' "$TMP/user.out"
echo "OK get_user_identity (bad id) -> 404"

# 6) identity lookup by unknown username -> 404
code=$(curl -s -o "$TMP/useru.out" -w '%{http_code}' "$API/auth/users/username/no-such-user-xyz")
test "$code" = "404"
rg -q 'RESOURCE_NOT_FOUND' "$TMP/useru.out"
echo "OK get_user_identity_by_username (unknown) -> 404"

# 7) login-link request -> 202 always (must not reveal account existence)
code=$(curl -s -o "$TMP/ll.out" -w '%{http_code}' -X POST "$API/auth/login-link" \
  -H 'Content-Type: application/json' \
  -d '{"email":"no-such-user@example.com"}')
test "$code" = "202"
rg -q '"accepted"\s*:\s*true' "$TMP/ll.out"
echo "OK login-link (unknown email) -> 202"

# 8) forgot-password request -> 202 always (same anti-enumeration contract)
code=$(curl -s -o "$TMP/fp.out" -w '%{http_code}' -X POST "$API/auth/forgot-password" \
  -H 'Content-Type: application/json' \
  -d '{"email":"no-such-user@example.com"}')
test "$code" = "202"
rg -q '"accepted"\s*:\s*true' "$TMP/fp.out"
echo "OK forgot-password (unknown email) -> 202"

# 9) login-link confirm with a bogus token -> 400
code=$(curl -s -o "$TMP/llc.out" -w '%{http_code}' -X POST "$API/auth/login-link/confirm" \
  -H 'Content-Type: application/json' \
  -d '{"token":"bogus-token"}')
test "$code" = "400"
rg -q 'INVALID_ARGUMENT|VALIDATION_ERROR' "$TMP/llc.out"
echo "OK login-link confirm (bogus token) -> 400"

# 10) reset-password with a bogus token -> 400
code=$(curl -s -o "$TMP/rp.out" -w '%{http_code}' -X POST "$API/auth/reset-password" \
  -H 'Content-Type: application/json' \
  -d '{"token":"bogus-token","newPassword":"SomeNewPass1"}')
test "$code" = "400"
rg -q 'INVALID_ARGUMENT|VALIDATION_ERROR' "$TMP/rp.out"
echo "OK reset-password (bogus token) -> 400"

# 11) authenticated endpoints without a token -> 401
for path in \
  "auth/session" \
  "auth/session-status" \
  "auth/logout" \
  "auth/access-rights" \
  "auth/verification-status" \
  "auth/resend-verification" \
  "auth/verify-password" \
  "auth/mail" \
  "auth/me" \
  "users/deadbeefdeadbeefdeadbeef"
do
  method=GET
  case "$path" in
    auth/logout|auth/resend-verification|auth/verify-password|auth/mail) method=POST ;;
    auth/me) method=PUT ;;
  esac
  code=$(curl -s -o "$TMP/unauth.out" -w '%{http_code}' -X "$method" "$API/$path")
  test "$code" = "401"
  rg -q 'Unauthorized' "$TMP/unauth.out"
  echo "OK $path (no token) -> 401"
done

# 12) change-password without a token -> 401 (query params, PUT)
code=$(curl -s -o "$TMP/cp.out" -w '%{http_code}' -X PUT \
  "$API/auth/change-password?currentPassword=old&newPassword=newpassword123")
test "$code" = "401"
rg -q 'Unauthorized' "$TMP/cp.out"
echo "OK change-password (no token) -> 401"

# 13) admin routes without a superadmin token -> 401
code=$(curl -s -o "$TMP/admin.out" -w '%{http_code}' -X POST "$API/auth/admin/register" \
  -H 'Content-Type: application/json' \
  -d '{"username":"x","email":"x@example.com","password":"LongPass1","name":"X"}')
test "$code" = "401"
rg -q 'Unauthorized' "$TMP/admin.out"
echo "OK admin/register (no token) -> 401"

echo
echo "All auth LXS smoke checks passed against $BASE_URL"
