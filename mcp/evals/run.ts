/**
 * Eval runner entrypoint.
 *
 *   npm run eval:mock     # no key, no binary — validates the harness itself
 *   npm run eval          # live: needs ANTHROPIC_API_KEY + CRYPTOFOLIO_BIN
 *
 * Prints a per-scenario report + aggregate pass-rate, and writes full transcripts
 * to evals/results/ for inspection (reading transcripts is part of the method —
 * aggregate scores hide failure modes).
 */

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runScenario, type Model, type Tools } from "./harness.js";
import { scenarios } from "./scenarios.js";
import { MockModel, MockTools } from "./mock.js";
import type { ScenarioResult, SuiteResult } from "./types.js";

async function buildDeps(mock: boolean): Promise<{
  model: Model;
  tools: Tools;
  setup: (s: (typeof scenarios)[number]) => Promise<void>;
  cleanup: () => Promise<void>;
}> {
  if (mock) {
    return {
      model: new MockModel(),
      tools: new MockTools(),
      setup: async () => {},
      cleanup: async () => {},
    };
  }

  // Live wiring is imported lazily so mock mode needs neither the Anthropic SDK
  // key nor a built binary.
  const real = await import("./real.js");
  const serverEntry = process.env.MCP_SERVER_ENTRY ?? join(process.cwd(), "dist", "index.js");
  const evalEnv = real.makeTempEnv();
  const tools = new real.McpTools(serverEntry, evalEnv.env);
  return {
    model: new real.RealModel(),
    tools,
    setup: async (s) => {
      if (s.setup) await real.runSetup(evalEnv, s.setup);
    },
    cleanup: async () => {
      await tools.close();
    },
  };
}

function summarize(results: ScenarioResult[]): SuiteResult {
  const passed = results.filter((r) => r.pass).length;
  const total = results.length;
  return { results, passed, total, passRate: total === 0 ? 0 : passed / total };
}

function report(suite: SuiteResult, mock: boolean): void {
  console.log(`\nCryptofolio agent evals (${mock ? "mock" : "live"})\n`);
  for (const r of suite.results) {
    console.log(`${r.pass ? "PASS" : "FAIL"}  ${r.id} — ${r.intent}`);
    if (!r.pass) {
      if (r.trace.error) console.log(`        ! ${r.trace.error}`);
      for (const c of r.checks.filter((c) => !c.pass)) {
        console.log(`        ✗ ${c.desc}`);
      }
    }
  }
  console.log(
    `\n${suite.passed}/${suite.total} scenarios passed (${(suite.passRate * 100).toFixed(0)}%)\n`
  );
}

async function main(): Promise<void> {
  const mock = process.argv.includes("--mock");
  const deps = await buildDeps(mock);

  const results: ScenarioResult[] = [];
  try {
    for (const scenario of scenarios) {
      await deps.setup(scenario);
      results.push(await runScenario(scenario, deps.model, deps.tools));
    }
  } finally {
    await deps.cleanup();
  }

  const suite = summarize(results);

  // Persist transcripts for inspection.
  const outDir = join(process.cwd(), "evals", "results");
  mkdirSync(outDir, { recursive: true });
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  writeFileSync(
    join(outDir, `${mock ? "mock" : "live"}-${stamp}.json`),
    JSON.stringify(suite, null, 2)
  );

  report(suite, mock);

  // Mock mode is a harness self-test: it MUST be all-green.
  if (mock && suite.passed !== suite.total) {
    console.error("Harness self-test failed — the runner/grader/scenarios are inconsistent.");
    process.exit(1);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
