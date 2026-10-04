/**
 * Unit tests for the CLI wrapper (src/cli.ts) — success-envelope and error
 * fallbacks. `execa` is mocked so no real binary is required.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("execa", () => ({ execa: vi.fn() }));

import { execa } from "execa";
import {
  CliError,
  runCli,
  runCliRaw,
  UNSTRUCTURED_OUTPUT_MESSAGE,
} from "../src/cli.js";

interface FakeResult {
  exitCode: number;
  stdout?: string;
  stderr?: string;
}

function mockResult(result: FakeResult): void {
  vi.mocked(execa).mockResolvedValueOnce({
    exitCode: result.exitCode,
    stdout: result.stdout ?? "",
    stderr: result.stderr ?? "",
    all: `${result.stdout ?? ""}${result.stderr ?? ""}`,
  } as never);
}

async function capture(promise: Promise<unknown>): Promise<unknown> {
  return promise.then(
    () => {
      throw new Error("expected the call to reject");
    },
    (err: unknown) => err
  );
}

describe("runCli success path", () => {
  beforeEach(() => vi.resetAllMocks());

  it("appends --json --quiet and parses JSON stdout", async () => {
    mockResult({ exitCode: 0, stdout: '{"total_realized":"1.50"}' });

    await expect(runCli(["pnl", "summary"])).resolves.toEqual({
      total_realized: "1.50",
    });
    expect(vi.mocked(execa)).toHaveBeenCalledWith(
      expect.any(String),
      ["pnl", "summary", "--json", "--quiet"],
      expect.objectContaining({ reject: false })
    );
  });

  it("returns null when stdout is empty or blank", async () => {
    mockResult({ exitCode: 0, stdout: "   \n" });
    await expect(runCli(["category", "add", "x"])).resolves.toBeNull();
  });

  it("does not leak raw non-JSON stdout as structured data", async () => {
    mockResult({
      exitCode: 0,
      stdout: "[OK] Sync complete! Processed 3 wallets",
    });

    const result = await runCli(["sync"]);

    expect(result).toEqual({ message: UNSTRUCTURED_OUTPUT_MESSAGE });
    expect(JSON.stringify(result)).not.toContain("Sync complete");
  });
});

describe("runCli error path", () => {
  beforeEach(() => vi.resetAllMocks());

  it("throws a well-formed CliError when stderr is empty", async () => {
    mockResult({ exitCode: 1, stdout: "", stderr: "" });

    const err = await capture(runCli(["tx", "list"]));

    expect(err).toBeInstanceOf(CliError);
    expect((err as CliError).exitCode).toBe(1);
    expect((err as CliError).message).toContain("unknown error");
    expect((err as CliError).message.length).toBeGreaterThan(0);
  });

  it("skips blank lines and reports the first real stderr line", async () => {
    mockResult({ exitCode: 2, stderr: "\n   \n  Error: bad account  \nmore" });

    const err = await capture(runCli(["tx", "list"]));

    expect((err as CliError).message).toContain("Error: bad account");
  });

  it("falls back to stdout when the CLI reports the failure there", async () => {
    mockResult({ exitCode: 1, stdout: "error: no such account\n", stderr: "" });

    const err = await capture(runCli(["account", "show"]));

    expect((err as CliError).stderr).toContain("no such account");
    expect((err as CliError).message).toContain("no such account");
  });

  it("wraps a spawn failure as exit 127 with a non-empty message and hint", async () => {
    vi.mocked(execa).mockRejectedValueOnce(new Error(""));

    const err = await capture(runCli(["pnl", "summary"]));

    expect(err).toBeInstanceOf(CliError);
    expect((err as CliError).exitCode).toBe(127);
    expect((err as CliError).message.length).toBeGreaterThan(0);
    expect((err as CliError).hint).toBeDefined();
  });

  it("maps a resolved ENOENT result to exit 127 with the binary hint", async () => {
    // execa with reject:false resolves (not throws) on ENOENT; exitCode is
    // undefined, so this must not fall through to "exit 1: unknown error".
    vi.mocked(execa).mockResolvedValueOnce({
      failed: true,
      timedOut: false,
      exitCode: undefined,
      stdout: "",
      stderr: "",
      shortMessage: "Command failed with ENOENT: cryptofolio x",
      all: "",
    } as never);

    const err = await capture(runCli(["pnl", "summary"]));

    expect(err).toBeInstanceOf(CliError);
    expect((err as CliError).exitCode).toBe(127);
    expect((err as CliError).message).toContain("ENOENT");
    expect((err as CliError).hint).toContain("CRYPTOFOLIO_BIN");
  });
});

describe("runCliRaw", () => {
  beforeEach(() => vi.resetAllMocks());

  it("returns trimmed raw stdout without --json", async () => {
    mockResult({ exitCode: 0, stdout: "  Sync complete  \n" });

    await expect(runCliRaw(["sync"])).resolves.toBe("Sync complete");
    expect(vi.mocked(execa)).toHaveBeenCalledWith(
      expect.any(String),
      ["sync", "--quiet"],
      expect.objectContaining({ reject: false })
    );
  });

  it("throws a well-formed CliError when stderr is empty", async () => {
    mockResult({ exitCode: 1, stdout: "", stderr: "" });

    const err = await capture(runCliRaw(["sync"]));

    expect(err).toBeInstanceOf(CliError);
    expect((err as CliError).message).toContain("unknown error");
  });

  it("maps a resolved ENOENT result to exit 127 with the binary hint", async () => {
    vi.mocked(execa).mockResolvedValueOnce({
      failed: true,
      timedOut: false,
      exitCode: undefined,
      stdout: "",
      stderr: "",
      shortMessage: "Command failed with ENOENT: cryptofolio sync",
      all: "",
    } as never);

    const err = await capture(runCliRaw(["sync"]));

    expect(err).toBeInstanceOf(CliError);
    expect((err as CliError).exitCode).toBe(127);
    expect((err as CliError).hint).toContain("CRYPTOFOLIO_BIN");
  });
});
