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

/**
 * Success envelope returned by runCli() when the command succeeded but printed
 * something that was not JSON (i.e. it does not implement --json). It is
 * deliberately a fixed marker rather than the raw stdout text, so callers can
 * never mistake unstructured output for a parsed result.
 */
export const UNSTRUCTURED_OUTPUT_MESSAGE =
  "Command completed (no structured output)";

const BINARY_HINT =
  "Ensure CRYPTOFOLIO_BIN points to the cryptofolio binary, or add it to your PATH.";

/** First non-blank line of stderr, or a non-empty fallback. */
function firstLine(text: string): string {
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.length > 0) return trimmed;
  }
  return "unknown error";
}

/** Message of a thrown value, guaranteed to be non-empty. */
function describeError(err: unknown): string {
  if (err instanceof Error) {
    const message = err.message.trim();
    if (message.length > 0) return message;
  }
  const text = String(err).trim();
  return text.length > 0 && text !== "undefined" && text !== "null"
    ? text
    : "unknown error";
}

/**
 * Pick the first non-blank diagnostic stream. Some commands report failures on
 * stdout, and an empty stderr must not hide that.
 */
function describeStreams(...streams: Array<string | undefined>): string {
  for (const stream of streams) {
    if (stream && stream.trim().length > 0) return stream;
  }
  return "";
}

/**
 * execa does not throw on a spawn failure when `reject: false` is set — it
 * resolves with `failed === true` and `exitCode === undefined`. Detect that so
 * a missing binary still surfaces as the actionable exit-127 + hint error
 * instead of degrading into a hint-less "exit 1: unknown error".
 */
function spawnFailure(
  result: {
    failed: boolean;
    timedOut: boolean;
    exitCode?: number | undefined;
    stdout?: string | undefined;
    stderr?: string | undefined;
    shortMessage?: string | undefined;
    originalMessage?: string | undefined;
  },
  command: string
): CliError | null {
  if (!result.failed || result.timedOut || result.exitCode !== undefined) {
    return null;
  }
  const detail =
    describeStreams(result.stderr, result.stdout) ||
    result.shortMessage ||
    result.originalMessage ||
    "failed to launch the cryptofolio binary";
  return new CliError(127, detail, command, BINARY_HINT);
}

export class CliError extends Error {
  constructor(
    public readonly exitCode: number,
    public readonly stderr: string,
    public readonly command: string,
    public readonly hint?: string
  ) {
    const short = firstLine(stderr);
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
    // execa throws on a spawn error it cannot represent as a result (e.g. an
    // option validation failure).
    const msg = describeError(err);
    throw new CliError(127, msg, args[0] ?? "unknown", BINARY_HINT);
  }

  const launchError = spawnFailure(result, args[0] ?? "unknown");
  if (launchError) throw launchError;

  if (result.exitCode !== 0) {
    const stderr = describeStreams(result.stderr, result.stdout);
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
    // Command succeeded but emitted non-JSON (command doesn't implement --json
    // yet). Return a fixed marker — never the raw text — so tools can report
    // success without presenting CLI prose as a structured result.
    return { message: UNSTRUCTURED_OUTPUT_MESSAGE };
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
    const msg = describeError(err);
    throw new CliError(127, msg, args[0] ?? "unknown", BINARY_HINT);
  }

  const launchError = spawnFailure(result, args[0] ?? "unknown");
  if (launchError) throw launchError;

  if (result.exitCode !== 0) {
    const stderr = describeStreams(result.stderr, result.stdout);
    throw new CliError(
      result.exitCode ?? 1,
      stderr,
      args[0] ?? "unknown"
    );
  }

  return (result.stdout ?? "").trim();
}
