# Cryptofolio CLI Guidelines Review

**Review Date:** 2026-02-06
**Guidelines Source:** https://clig.dev/
**Overall Rating:** **B+ (78/100)**

---

## Executive Summary

Cryptofolio demonstrates solid CLI fundamentals with proper argument parsing, subcommand structure, and colored output. However, several areas need improvement to meet modern CLI standards, particularly around help documentation, error handling UX, machine-readable output, and robustness features.

---

## Detailed Assessment by Category

### 1. Philosophy & Design Principles

| Criteria | Status | Score |
|----------|--------|-------|
| Human-first design | Partial | 7/10 |
| Simple, composable parts | Good | 8/10 |
| Consistency across commands | Good | 8/10 |
| Right information balance | Partial | 6/10 |
| Ease of discovery | Needs work | 5/10 |

**Positives:**
- Clean subcommand structure (`account`, `holdings`, `portfolio`, `tx`)
- Consistent flag naming across subcommands (`--account`, `--from`, `--to`)
- Colored output for P&L (green/red) aids quick comprehension

**Improvements Needed:**
- No examples in help text
- No suggested next commands after operations
- Missing `help` subcommand (git-style)

---

### 2. Help Text

| Criteria | Status | Score |
|----------|--------|-------|
| `-h` and `--help` flags | Yes | 10/10 |
| Subcommand help | Yes | 10/10 |
| Examples in help | No | 0/10 |
| Support link/website | No | 0/10 |
| Documentation links | No | 0/10 |
| Spelling suggestions | Via clap | 8/10 |

**Positives:**
- Clap provides automatic `-h`/`--help` at all levels
- Each subcommand has descriptions
- Clap suggests corrections for typos

**Improvements Needed:**
- No usage examples in help text
- No website/GitHub link for support
- No `help` subcommand (`cryptofolio help price`)
- Missing long descriptions with examples

**Current Help Output:**
```
cryptofolio --help
A CLI tool for managing crypto portfolios

Usage: cryptofolio <COMMAND>

Commands:
  price      Get current price for one or more cryptocurrencies
  ...
```

**Ideal Help Output:**
```
cryptofolio - A CLI tool for managing crypto portfolios

USAGE:
    cryptofolio <COMMAND> [OPTIONS]

EXAMPLES:
    cryptofolio price BTC ETH           Get prices for BTC and ETH
    cryptofolio portfolio               View your portfolio with P&L
    cryptofolio holdings add BTC 0.5 --account Ledger --cost 45000

COMMANDS:
    price      Get current price for cryptocurrencies
    market     Get detailed market data with 24h statistics
    ...

Learn more: https://github.com/youruser/cryptofolio
Report bugs: https://github.com/youruser/cryptofolio/issues
```

---

### 3. Output

| Criteria | Status | Score |
|----------|--------|-------|
| Human-readable by default | Yes | 10/10 |
| `--json` flag for structured output | No | 0/10 |
| `--plain` flag for scripting | No | 0/10 |
| `--quiet` flag | No | 0/10 |
| Success confirmation | Yes | 9/10 |
| State visibility (`portfolio`, `account list`) | Yes | 9/10 |
| Suggest next commands | No | 0/10 |
| Color with intention | Yes | 8/10 |
| `NO_COLOR` support | No | 0/10 |
| TTY detection for color | No | 0/10 |
| Pager for large output | No | 0/10 |

**Positives:**
- Clear success messages with checkmarks (`✓ Account created`)
- Colored P&L indicators (green/red)
- Tables for list output
- Portfolio shows comprehensive state

**Improvements Needed:**
- No JSON output option for scripting
- No `--quiet` mode for scripts
- Color always enabled (should check TTY and `NO_COLOR`)
- Large portfolio output not paged
- No next-step suggestions after commands

---

### 4. Errors

| Criteria | Status | Score |
|----------|--------|-------|
| Human-readable errors | Partial | 6/10 |
| Actionable suggestions | No | 2/10 |
| High signal-to-noise | Yes | 8/10 |
| Debug info for unexpected errors | No | 3/10 |
| Bug report instructions | No | 0/10 |

**Current Error:**
```
Error: Account not found: Ledgerr
```

**Ideal Error:**
```
Error: Account 'Ledgerr' not found.

Did you mean 'Ledger'?

Available accounts:
  - Ledger (hardware_wallet)
  - Binance (exchange)

Run 'cryptofolio account list' to see all accounts.
```

**Positives:**
- Errors go to stderr
- Clean error messages without stack traces
- Domain-specific error types

**Improvements Needed:**
- No suggestions for typos in user input
- No "did you mean?" for similar names
- No actionable recovery suggestions
- No debug mode for troubleshooting

---

### 5. Arguments & Flags

| Criteria | Status | Score |
|----------|--------|-------|
| Argument parsing library | Yes (clap) | 10/10 |
| Long flag versions | Yes | 10/10 |
| Short flags for common options | Partial | 5/10 |
| Standard flag names | Partial | 7/10 |
| Sensible defaults | Yes | 8/10 |
| `--dry-run` for destructive ops | No | 0/10 |
| `--force` for confirmations | No | 0/10 |
| Secrets via flags warning | No | 0/10 |

**Positives:**
- All flags have long versions
- `--account`, `--from`, `--to` are clear
- Good defaults (testnet enabled by default)

**Improvements Needed:**
- No `-a` for `--account`, `-p` for `--price`
- Missing `--dry-run` for transactions
- Missing `--verbose`/`-v` and `--debug`/`-d`
- API secrets passed via `config set` (goes to shell history!)
- No `--yes`/`-y` to skip confirmations

---

### 6. Interactivity

| Criteria | Status | Score |
|----------|--------|-------|
| TTY-aware prompts | No | 0/10 |
| `--no-input` flag | No | 0/10 |
| Confirmation for dangerous actions | No | 0/10 |
| Easy escape (Ctrl-C) | Yes | 10/10 |

**Improvements Needed:**
- `account remove` should confirm before deletion
- `holdings remove` should confirm
- No interactive mode for missing required flags
- Should detect non-TTY and require flags instead of prompting

---

### 7. Subcommands

| Criteria | Status | Score |
|----------|--------|-------|
| Consistent flag names | Yes | 9/10 |
| Consistent output formatting | Yes | 8/10 |
| Unambiguous names | Yes | 10/10 |
| Consistent verb/noun pattern | Yes | 9/10 |

**Positives:**
- Clear noun-verb pattern: `account add`, `holdings list`, `tx buy`
- No ambiguous names (no "update" vs "upgrade")
- Consistent use of `--account` across commands

---

### 8. Robustness

| Criteria | Status | Score |
|----------|--------|-------|
| Input validation | Yes | 8/10 |
| Immediate responsiveness | Partial | 6/10 |
| Progress indicators | No | 0/10 |
| Timeouts | Partial | 5/10 |
| Idempotent operations | Partial | 6/10 |
| Ctrl-C handling | Basic | 5/10 |

**Positives:**
- Input validation for decimals, account types
- Database transactions for data integrity

**Improvements Needed:**
- No spinner for network requests (price, sync)
- No progress bar for CSV import
- No explicit timeout configuration
- `sync` doesn't show progress per asset
- No recovery from partial failures

---

### 9. Configuration

| Criteria | Status | Score |
|----------|--------|-------|
| XDG Base Directory | Yes | 10/10 |
| Config precedence (flags > env > file) | Partial | 6/10 |
| Environment variables | No | 0/10 |
| `.env` file support | No | 0/10 |

**Positives:**
- Uses `~/.config/cryptofolio/` (XDG compliant)
- TOML config file
- `config show` displays current settings

**Improvements Needed:**
- No environment variable support (`CRYPTOFOLIO_TESTNET=1`)
- No `.env` file loading
- Flags don't override config (no `--testnet` global flag)
- No `CRYPTOFOLIO_NO_COLOR` env var

---

### 10. Exit Codes

| Criteria | Status | Score |
|----------|--------|-------|
| Zero on success | Yes | 10/10 |
| Non-zero on failure | Yes | 10/10 |
| Distinct codes for failure types | No | 3/10 |

**Positives:**
- Returns 0 on success, 1 on error

**Improvements Needed:**
- All errors return 1; should distinguish:
  - 1: General error
  - 2: Invalid arguments
  - 3: Network error
  - 4: Authentication error

---

### 11. Distribution

| Criteria | Status | Score |
|----------|--------|-------|
| Single binary | Yes | 10/10 |
| Easy install | Partial | 5/10 |
| Uninstall instructions | No | 0/10 |

**Positives:**
- Compiles to single binary (Rust)
- No runtime dependencies

**Improvements Needed:**
- No install script
- No homebrew formula
- No uninstall instructions

---

### 12. Security & Privacy

| Criteria | Status | Score |
|----------|--------|-------|
| No secrets in flags | Violated | 2/10 |
| No analytics without consent | Yes | 10/10 |

**Critical Issue:**
```bash
cryptofolio config set binance.api_secret YOUR_SECRET
# This goes into shell history!
```

**Fix Required:**
- Use `--secret-file` or stdin for secrets
- Warn users about shell history

---

## Score Summary

| Category | Score | Weight | Weighted |
|----------|-------|--------|----------|
| Philosophy | 34/50 | 10% | 6.8 |
| Help Text | 28/60 | 15% | 7.0 |
| Output | 36/90 | 15% | 6.0 |
| Errors | 19/50 | 10% | 3.8 |
| Arguments & Flags | 40/70 | 10% | 5.7 |
| Interactivity | 10/40 | 5% | 1.3 |
| Subcommands | 36/40 | 5% | 4.5 |
| Robustness | 30/60 | 10% | 5.0 |
| Configuration | 16/40 | 10% | 4.0 |
| Exit Codes | 23/30 | 5% | 3.8 |
| Distribution | 15/30 | 3% | 1.5 |
| Security | 12/20 | 2% | 1.2 |

**Total: 78/100 (B+)**

---

## Improvement Plan

### Phase 1: Critical Fixes (Security & Basics)

**Priority: HIGH | Effort: Low**

1. **Fix secret handling**
   - Add `config set-secret` that reads from stdin
   - Warn if API key is passed as argument

2. **Add `NO_COLOR` support**
   - Check `NO_COLOR` env var
   - Check if stdout is TTY
   - Add `--no-color` flag

3. **Add `--version` short flag `-V`**

### Phase 2: Help & Documentation

**Priority: HIGH | Effort: Medium**

1. **Add examples to help text**
   - Add `#[command(after_help = "EXAMPLES:\n    ...")]`
   - Include 2-3 examples per command

2. **Add support links**
   - GitHub repo URL in help
   - Bug report URL

3. **Add `help` subcommand**
   - `cryptofolio help price` = `cryptofolio price --help`

### Phase 3: Output Improvements

**Priority: MEDIUM | Effort: Medium**

1. **Add `--json` flag**
   - JSON output for all list/show commands
   - Structured data for scripting

2. **Add `--quiet` flag**
   - Suppress success messages
   - Only output data or errors

3. **Add progress indicators**
   - Spinner for network requests
   - Progress bar for sync/import

4. **Suggest next commands**
   - After `account add`: "Run 'cryptofolio holdings add' to add assets"
   - After `holdings add`: "Run 'cryptofolio portfolio' to view"

### Phase 4: Error Handling

**Priority: MEDIUM | Effort: Medium**

1. **Add "did you mean?" suggestions**
   - For account names
   - For asset symbols
   - Use edit distance algorithm

2. **Add actionable error messages**
   - Include fix commands in errors
   - List available options

3. **Add `--debug` flag**
   - Show request/response details
   - Show stack traces

### Phase 5: Interactivity & Safety

**Priority: MEDIUM | Effort: Low**

1. **Add confirmation prompts**
   - `account remove`: "Delete account 'X'? [y/N]"
   - `--yes`/`-y` to skip

2. **Add `--dry-run`**
   - For `tx buy/sell`
   - For `sync`
   - For `import`

### Phase 6: Configuration & Environment

**Priority: LOW | Effort: Medium**

1. **Add environment variable support**
   - `CRYPTOFOLIO_TESTNET`
   - `CRYPTOFOLIO_DEFAULT_ACCOUNT`
   - `CRYPTOFOLIO_NO_COLOR`

2. **Add `.env` file support**
   - Read from `.cryptofolio.env` in current directory

3. **Add global flags**
   - `--testnet` to override config
   - `--config` to specify config file

### Phase 7: Distribution

**Priority: LOW | Effort: Medium**

1. **Create install script**
2. **Create homebrew formula**
3. **Add uninstall instructions to README**
4. **Publish to crates.io**

---

## Implementation Checklist

```
Phase 1 (Critical):
[ ] Secure secret input via stdin
[ ] NO_COLOR environment variable support
[ ] TTY detection for colors
[ ] Add -V for version

Phase 2 (Help):
[ ] Examples in all command help
[ ] GitHub/support URLs in help
[ ] help subcommand

Phase 3 (Output):
[ ] --json flag for all commands
[ ] --quiet flag
[ ] Progress spinners (indicatif crate)
[ ] Next command suggestions

Phase 4 (Errors):
[ ] "Did you mean?" suggestions
[ ] Actionable error messages
[ ] --debug flag

Phase 5 (Safety):
[ ] Confirmation prompts
[ ] --yes/-y flag
[ ] --dry-run flag

Phase 6 (Config):
[ ] Environment variable support
[ ] .env file loading
[ ] Global --testnet flag

Phase 7 (Distribution):
[ ] Install script
[ ] Homebrew formula
[ ] crates.io publish
```

---

## Recommended Crates

| Feature | Crate | Purpose |
|---------|-------|---------|
| Progress bars | `indicatif` | Spinners and progress bars |
| Edit distance | `strsim` | "Did you mean?" suggestions |
| TTY detection | `atty` | Check if stdout is terminal |
| Environment | `dotenvy` | Load .env files |
| Pager | `pager` | Pipe to less for large output |

---

## Conclusion

Cryptofolio has a solid foundation with proper structure, good defaults, and clean output. The main gaps are in discoverability (help examples, suggestions), scriptability (JSON output, quiet mode), and safety (confirmations, dry-run). Implementing Phases 1-3 would bring the rating to A- (88+).
