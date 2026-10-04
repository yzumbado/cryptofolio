#!/usr/bin/env bash
#
# portfolio-refresh.sh — daily portfolio refresh (T13).
#
# WHAT IT DOES
#   1. Syncs every registered blockchain wallet (software + hardware wallets),
#      incrementally — only new activity since the last sync watermark.
#   2. Refreshes the portfolio valuation, which fetches a live price for every
#      held asset (including DeFi/Earn receipt underlyings) and logs the result.
#   3. Appends everything to a timestamped log and exits non-zero on failure so
#      launchd can flag the run.
#
# HOW TO INSTALL (macOS, launchd)
#   1. Build/refresh the binary first — a stale `~/.local/bin/cryptofolio`
#      silently syncs nothing new:
#        cargo build --release
#        cp target/release/cryptofolio ~/.local/bin/cryptofolio
#      (Or point CRYPTOFOLIO_BIN at the repo build; see ENV below.)
#   2. Install this script somewhere stable, e.g.:
#        mkdir -p ~/.local/share/cryptofolio
#        cp scripts/portfolio-refresh.sh ~/.local/share/cryptofolio/
#        chmod +x ~/.local/share/cryptofolio/portfolio-refresh.sh
#   3. Install the launchd job (the plist references the path above):
#        cp scripts/com.cryptofolio.refresh.plist ~/Library/LaunchAgents/
#        launchctl load ~/Library/LaunchAgents/com.cryptofolio.refresh.plist
#      (To undo: `launchctl unload ~/Library/LaunchAgents/com.cryptofolio.refresh.plist`
#       and delete the plist and script.)
#   4. Run it once by hand to confirm:
#        ~/.local/share/cryptofolio/portfolio-refresh.sh
#
# ENVIRONMENT
#   CRYPTOFOLIO_BIN       Path to the CLI binary.
#                         Default: $HOME/.local/bin/cryptofolio
#                         Set this to the repo build, e.g.
#                         /path/to/cryptofolio/target/release/cryptofolio, if you
#                         do not want to copy the binary into ~/.local/bin.
#   CRYPTOFOLIO_LOG_DIR   Directory for the run logs.
#                         Default: $HOME/Library/Logs/cryptofolio
#
# SECURITY / NO SECRETS
#   This script contains no keys, addresses, or account names. Credentials are
#   read by the CLI from its own config/keychain, exactly as in an interactive run.
#
set -euo pipefail

# launchd runs with a minimal PATH; pin the standard system locations so the
# tools below (date, mkdir, ...) resolve.
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"

CRYPTOFOLIO_BIN="${CRYPTOFOLIO_BIN:-$HOME/.local/bin/cryptofolio}"
LOG_DIR="${CRYPTOFOLIO_LOG_DIR:-$HOME/Library/Logs/cryptofolio}"

mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/portfolio-refresh-$(date +%Y-%m-%d).log"

# Send all subsequent stdout/stderr to the timestamped log.
exec >>"$LOG_FILE" 2>&1

log() { printf '[%s] %s\n' "$(date '+%Y-%m-%dT%H:%M:%S%z')" "$*"; }

# Run one step, log BEGIN/OK/FAIL, and abort the whole run on failure so launchd
# records a non-zero exit. Commands inside `if` are exempt from `set -e`, which
# is what lets us log the status instead of dying silently.
run_step() {
    local label="$1"
    shift
    log "BEGIN ${label}: $*"
    if "$@"; then
        log "OK    ${label}"
    else
        local rc=$?
        log "FAIL  ${label} (exit ${rc})"
        exit "$rc"
    fi
}

log "=== portfolio refresh starting (pid $$) ==="
log "binary:  ${CRYPTOFOLIO_BIN}"
log "logfile: ${LOG_FILE}"

if [ ! -x "$CRYPTOFOLIO_BIN" ]; then
    log "ERROR: binary not found or not executable: ${CRYPTOFOLIO_BIN}"
    log "       Set CRYPTOFOLIO_BIN to your build, or run: cargo build --release"
    exit 127
fi

# 1. Sync all blockchain wallets. Incremental by default: the CLI only passes
#    full_history when --import-history is given (src/cli/commands/wallet.rs:743),
#    so we deliberately omit it to import just new activity.
#    NOTE: per-address provider errors are reported inside the run and the
#    command still exits 0 (src/cli/commands/wallet.rs:747-765). Check the log
#    and `cryptofolio audit errors` for those; this script only fails the run
#    when the command itself fails.
run_step "wallet sync (all, incremental)" "$CRYPTOFOLIO_BIN" wallet sync --all

# 2. Refresh prices. There is no persistent price-refresh/cache command: nothing
#    writes fetched prices to the DB (the only rate table is `exchange_rates`,
#    populated manually via `currency set-rate`). `cryptofolio portfolio --json`
#    is the command that discovers every held asset and fetches a live price for
#    each through the shared pricing pipeline. Its JSON (fresh total value) lands
#    in this log; nothing is cached between runs. For a single-asset spot check
#    you can add e.g.:  "$CRYPTOFOLIO_BIN" price BTC
run_step "portfolio price refresh" "$CRYPTOFOLIO_BIN" portfolio --json

log "=== portfolio refresh done ==="
