/**
 * Unit tests for the eval grader — pure logic, no key/binary needed.
 * Proves the heart of the LLM-eval harness is correct so its verdicts are trustworthy.
 */

import { describe, it, expect } from "vitest";
import { gradeChecks, gradeScenario } from "../../evals/grader.js";
import type { Scenario, Trace } from "../../evals/types.js";

const trace = (over: Partial<Trace> = {}): Trace => ({
  calls: [],
  finalText: "",
  ...over,
});

describe("gradeChecks", () => {
  it("toolCalled passes when the tool was called", () => {
    const t = trace({ calls: [{ name: "cryptofolio_get_portfolio", input: {} }] });
    const [r] = gradeChecks(
      [{ kind: "toolCalled", tool: "cryptofolio_get_portfolio", desc: "x" }],
      t
    );
    expect(r?.pass).toBe(true);
  });

  it("toolCalled honors the where predicate", () => {
    const t = trace({
      calls: [{ name: "cryptofolio_record_transaction", input: { cost_basis_only: true } }],
    });
    const pass = gradeChecks(
      [
        {
          kind: "toolCalled",
          tool: "cryptofolio_record_transaction",
          where: (i) => i.cost_basis_only === true,
          desc: "x",
        },
      ],
      t
    );
    expect(pass[0]?.pass).toBe(true);

    const fail = gradeChecks(
      [
        {
          kind: "toolCalled",
          tool: "cryptofolio_record_transaction",
          where: (i) => i.cost_basis_only === true,
          desc: "x",
        },
      ],
      trace({ calls: [{ name: "cryptofolio_record_transaction", input: {} }] })
    );
    expect(fail[0]?.pass).toBe(false);
  });

  it("toolNotCalled passes only when absent", () => {
    const t = trace({ calls: [{ name: "cryptofolio_get_prices", input: {} }] });
    expect(
      gradeChecks([{ kind: "toolNotCalled", tool: "cryptofolio_get_portfolio", desc: "x" }], t)[0]
        ?.pass
    ).toBe(true);
    expect(
      gradeChecks([{ kind: "toolNotCalled", tool: "cryptofolio_get_prices", desc: "x" }], t)[0]?.pass
    ).toBe(false);
  });

  it("mentions is case-insensitive and any-of", () => {
    const t = trace({ finalText: "Your portfolio is EMPTY right now." });
    expect(
      gradeChecks([{ kind: "mentions", any: ["empty", "nothing"], desc: "x" }], t)[0]?.pass
    ).toBe(true);
  });

  it("notMentions fails when a forbidden substring appears", () => {
    const t = trace({ finalText: "It's approximately $12,400." });
    expect(
      gradeChecks([{ kind: "notMentions", all: ["approximately $"], desc: "x" }], t)[0]?.pass
    ).toBe(false);
  });
});

describe("gradeScenario", () => {
  const scenario: Scenario = {
    id: "s1",
    intent: "demo",
    prompt: "p",
    rubric: [{ kind: "toolCalled", tool: "cryptofolio_get_portfolio", desc: "calls portfolio" }],
  };

  it("passes when every check passes", () => {
    const r = gradeScenario(
      scenario,
      trace({ calls: [{ name: "cryptofolio_get_portfolio", input: {} }] })
    );
    expect(r.pass).toBe(true);
  });

  it("fails the whole scenario when the run errored, even if checks pass", () => {
    const r = gradeScenario(
      scenario,
      trace({ calls: [{ name: "cryptofolio_get_portfolio", input: {} }], error: "max steps" })
    );
    expect(r.pass).toBe(false);
  });
});
