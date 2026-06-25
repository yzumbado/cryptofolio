/**
 * Pure grading logic — no I/O, fully unit-testable.
 *
 * Given a scenario's tool-call trace + final answer, evaluate each rubric check.
 * This is the heart of the harness and is covered by grader.test.ts.
 */

import type { Check, CheckResult, Scenario, ScenarioResult, Trace } from "./types.js";

function evalCheck(check: Check, trace: Trace): boolean {
  switch (check.kind) {
    case "toolCalled":
      return trace.calls.some(
        (c) => c.name === check.tool && (check.where ? check.where(c.input) : true)
      );
    case "toolNotCalled":
      return !trace.calls.some((c) => c.name === check.tool);
    case "mentions": {
      const text = trace.finalText.toLowerCase();
      return check.any.some((s) => text.includes(s.toLowerCase()));
    }
    case "notMentions": {
      const text = trace.finalText.toLowerCase();
      return check.all.every((s) => !text.includes(s.toLowerCase()));
    }
  }
}

export function gradeChecks(rubric: Check[], trace: Trace): CheckResult[] {
  return rubric.map((check) => ({ desc: check.desc, pass: evalCheck(check, trace) }));
}

export function gradeScenario(scenario: Scenario, trace: Trace): ScenarioResult {
  const checks = gradeChecks(scenario.rubric, trace);
  // A run that errored out fails regardless of individual checks.
  const pass = trace.error === undefined && checks.every((c) => c.pass);
  return {
    id: scenario.id,
    intent: scenario.intent,
    pass,
    checks,
    trace,
  };
}
