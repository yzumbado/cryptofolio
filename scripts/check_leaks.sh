#!/usr/bin/env bash
# Leak guard — deterministic gate against committing secrets or personal data.
#
# Rules:
#   SECRETS   — API keys, JWTs, WIF private keys, raw 64-hex keys. Everywhere.
#   PERSONAL  — machine paths, user names, wallet names. Everywhere.
#   MONEY     — dollar figures. Banned (zero tolerance) only in the files
#               listed in MONEY_FORBIDDEN, where real portfolio figures live;
#               illustrative examples elsewhere in docs are legitimate.
#
# Any hit prints "path:line:RULE" (never the matched content, so CI logs don't
# leak) and exits 1. Known-false positives are excluded deterministically by
# `.leaks-allowlist` entries of the form "path:line" (one per line).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

SECRETS='sk-[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{36}|eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}|5[HJK][1-9A-HJ-NP-Za-km-z]{50,}|[0-9a-f]{64}'
# Machine paths, wallet names, full name. (The GitHub username alone is
# intentional in repo URLs, so it is not a marker here.)
PERSONAL='/Users/[A-Za-z0-9._-]+|Avioneta|Keystone|Yoel'
MONEY='\$[0-9]{2,3}(,[0-9]{3})+(\.[0-9]+)?'
# Files where dollar figures are never acceptable (real position data lives here).
MONEY_FORBIDDEN='docs/BACKLOG.md|docs/MINING_ASSET_ACCOUNTING.md|docs/design/'

ALLOWLIST=""
if [ -f .leaks-allowlist ]; then
  ALLOWLIST=$(grep -v '^\s*#' .leaks-allowlist | grep -v '^\s*$' || true)
fi

allowed() {
  [ -z "$ALLOWLIST" ] && return 1
  echo "$ALLOWLIST" | grep -qxF "$1"
}

fail=0
hits=""
scan() { # $1=file $2=pattern $3=rulename
  while IFS=: read -r ln _; do
    [ -n "$ln" ] || continue
    key="$1:$ln"
    if allowed "$key"; then continue; fi
    hits="$hits$key: $3\n"
  done < <(grep -nE "$2" "$1" 2>/dev/null || true)
}

while IFS= read -r f; do
  case "$f" in
    *.lock|*.png|*.jpg|*.svg|*.ico|*.woff|*.woff2|*.pdf) continue ;;
    # The guard's own files contain the patterns themselves
    scripts/check_leaks.sh|.leaks-allowlist|.github/workflows/leak-check.yml) continue ;;
  esac
  if ! grep -Ilq '' "$f" 2>/dev/null; then continue; fi

  scan "$f" "$SECRETS" SECRETS
  scan "$f" "$PERSONAL" PERSONAL
  if echo "$f" | grep -qE "^($MONEY_FORBIDDEN)"; then
    scan "$f" "$MONEY" MONEY
  fi
done < <(git ls-files)

if [ -n "$hits" ]; then
  printf '%b' "$hits"
  echo "Leak guard failed: $(printf '%b' "$hits" | grep -c ':') potential leak(s)."
  echo "Fix the content, or add the exact 'path:line' to .leaks-allowlist if it is a verified false positive."
  exit 1
fi
echo "Leak guard passed."
