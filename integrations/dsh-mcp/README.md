# DSH MCP bundle

Configuration-only DeepSeek Harness bundle connecting the cryptofolio MCP server
(23 `cryptofolio_*` tools) to a DSH profile over stdio.

## Setup

1. Build prerequisites (fresh artifacts):
   ```bash
   cargo build --release
   (cd mcp && npm run build)
   ```
2. Create the local patch from the template and replace `REPO_ROOT` with the
   absolute path to this checkout:
   ```bash
   cp cordis.patch.example.yml cordis.patch.yml
   ```
   `cordis.patch.yml` is **gitignored** — it contains machine-specific paths.
3. Install through DSH's plugin manager with this directory as the bundle target
   (or equivalent profile management).

Tools then appear in DSH sessions as `mcp__cryptofolio__*`.
