/**
 * Types for the LLM-in-the-loop eval harness.
 *
 * Design (per Anthropic agent-eval guidance):
 * - Scenarios are realistic, end-to-end user requests.
 * - Grading is primarily on the *tool-call trace* + final answer (deterministic),
 *   with optional narrative checks. We exercise the REAL MCP tools.
 * - The runner is parameterized over `Model` and `Tools` interfaces so it can run
 *   live (Anthropic + MCP client) or in `--mock` mode (no key, no binary) — which
 *   makes the harness itself unit-testable.
 */

export interface ToolDef {
  name: string;
  description: string;
  /** JSON Schema for the tool input (from MCP `listTools`). */
  inputSchema: Record<string, unknown>;
}

export interface ToolCall {
  name: string;
  input: Record<string, unknown>;
}

export interface ToolResult {
  name: string;
  /** Text payload returned to the model. */
  output: string;
  isError: boolean;
}

/** Everything the grader needs about one scenario run. */
export interface Trace {
  calls: ToolCall[];
  finalText: string;
  /** Set if the run aborted (max steps, model/transport error). */
  error?: string;
}

/**
 * A single rubric check. Kept as a discriminated union so grading is a pure,
 * exhaustively-checked function.
 */
export type Check =
  | {
      kind: "toolCalled";
      tool: string;
      /** Optional predicate over the call input (e.g. cost_basis_only === true). */
      where?: (input: Record<string, unknown>) => boolean;
      desc: string;
    }
  | { kind: "toolNotCalled"; tool: string; desc: string }
  /** Final answer contains at least one of these substrings (case-insensitive). */
  | { kind: "mentions"; any: string[]; desc: string }
  /** Final answer contains NONE of these substrings (case-insensitive). */
  | { kind: "notMentions"; all: string[]; desc: string };

export interface Scenario {
  id: string;
  /** Plain-English description of what the user is trying to do. */
  intent: string;
  /** The user's message to the agent. */
  prompt: string;
  /**
   * CLI argument arrays run against the temp DB before the scenario, to seed
   * state (e.g. create an account). Ignored in mock mode.
   */
  setup?: string[][];
  rubric: Check[];
}

export interface CheckResult {
  desc: string;
  pass: boolean;
}

export interface ScenarioResult {
  id: string;
  intent: string;
  pass: boolean;
  checks: CheckResult[];
  trace: Trace;
}

export interface SuiteResult {
  results: ScenarioResult[];
  passed: number;
  total: number;
  /** 0..1 */
  passRate: number;
}
