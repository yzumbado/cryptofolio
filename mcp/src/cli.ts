/**
 * CLI wrapper — single choke point for all subprocess calls to the cryptofolio binary.
 *
 * Design:
 *  - Resolves binary from CRYPTOFOLIO_BIN env var, falls back to 'cryptofolio' on PATH.
 *  - Appends --json and --quiet to every call so output is machine-parseable and clean.
 *  - runCli()    → parses stdout as JSON, throws CliError on non-zero exit.
 *  - runCliRaw() → returns raw stdout text, throws CliError on non-zero exit.
 *                  Use for commands that do not implement --json output (e.g. sync, category add).
 */

import { execa } from "execa";

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

export class CliError extends Error {
  constructor(
    public readonly exitCode: number,
    public readonly stderr: string,
    public readonly command: string,
    public readonly hint?: string
  ) {
    const short = stderr.split("\n")[0] ?? "unknown error";
    super(`cryptofolio ${command} failed (exit ${exitCode}): ${short}`);
    this.name = "CliError";
  }
}

// ---------------------------------------------------------------------------
// Binary resolution
// ---------------------------------------------------------------------------

function resolveBin(): string {
  return process.env["CRYPTOFOLIO_BIN"] ?? "cryptofolio";
}

// ---------------------------------------------------------------------------
// Timeout constants (ms)
// ---------------------------------------------------------------------------

const DEFAULT_TIMEOUT_MS = 30_000;
export const SYNC_TIMEOUT_MS = 120_000; // wallet sync can be slow

// ---------------------------------------------------------------------------
// runCli — expects JSON output
// ---------------------------------------------------------------------------

/**
 * Run the cryptofolio binary with `--json --quiet` appended.
 * Returns the parsed JSON output (or null if stdout is empty).
 * Throws CliError on non-zero exit code.
 */
export async function runCli(
  args: string[],
  timeoutMs: number = DEFAULT_TIMEOUT_MS
): Promise<unknown> {
  const bin = resolveBin();
  const fullArgs = [...args, "--json", "--quiet"];

  let result;
  try {
    result = await execa(bin, fullArgs, {
      reject: false,
      timeout: timeoutMs,
      all: true,
    });
  } catch (err: unknown) {
    // execa throws on ENOENT (binary not found) or timeout
    const msg =
      err instanceof Error ? err.message : String(err);
    throw new CliError(
      127,
      msg,
      args[0] ?? "unknown",
      `Ensure CRYPTOFOLIO_BIN points to the cryptofolio binary, or add it to your PATH.`
    );
  }

  if (result.exitCode !== 0) {
    const stderr = result.stderr ?? result.stdout ?? "";
    throw new CliError(
      result.exitCode ?? 1,
      stderr,
      args[0] ?? "unknown"
    );
  }

  const stdout = (result.stdout ?? "").trim();
  if (!stdout) return null;

  try {
    return JSON.parse(stdout) as unknown;
  } catch {
    // Command succeeded but emitted non-JSON (command doesn't implement --json yet).
    // Return a wrapper so tools can still report success.
    return { message: stdout };
  }
}

// ---------------------------------------------------------------------------
// runCliRaw — does NOT append --json (for commands without JSON output)
// ---------------------------------------------------------------------------

/**
 * Run the cryptofolio binary with `--quiet` only (no --json).
 * Returns raw stdout text on success.
 * Throws CliError on non-zero exit code.
 */
export async function runCliRaw(
  args: string[],
  timeoutMs: number = DEFAULT_TIMEOUT_MS
): Promise<string> {
  const bin = resolveBin();
  const fullArgs = [...args, "--quiet"];

  let result;
  try {
    result = await execa(bin, fullArgs, {
      reject: false,
      timeout: timeoutMs,
      all: true,
    });
  } catch (err: unknown) {
    const msg =
      err instanceof Error ? err.message : String(err);
    throw new CliError(
      127,
      msg,
      args[0] ?? "unknown",
      `Ensure CRYPTOFOLIO_BIN points to the cryptofolio binary, or add it to your PATH.`
    );
  }

  if (result.exitCode !== 0) {
    const stderr = result.stderr ?? result.stdout ?? "";
    throw new CliError(
      result.exitCode ?? 1,
      stderr,
      args[0] ?? "unknown"
    );
  }

  return (result.stdout ?? "").trim();
}
