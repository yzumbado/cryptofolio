#!/usr/bin/env bash
# Known limitation: excludes everything after the first #[cfg(test)] in a file.
# Good enough as a ratchet; do not over-engineer.
set -euo pipefail
BASELINE=8
COUNT=$(python3 - <<'PY'
import glob
total=0
for f in glob.glob('src/**/*.rs', recursive=True):
    s=open(f).read()
    idx=s.find('#[cfg(test)]')
    prod = s if idx==-1 else s[:idx]
    total += prod.count('.unwrap()')+prod.count('.expect(')
print(total)
PY
)
echo "Production unwrap/expect count: $COUNT (baseline $BASELINE)"
if [ "$COUNT" -gt "$BASELINE" ]; then
  echo "::error::unwrap/expect regression: $COUNT > $BASELINE. Return Result instead."
  exit 1
fi
