/**
 * The agent loop, parameterized over `Model` and `Tools` interfaces.
 *
 * This is the dependency-injection seam: live runs wire a real Anthropic model
 * and a real MCP-client tool bridge (real.ts); mock runs wire scripted stand-ins
 * (mock.ts). The loop logic is identical either way, so mock-mode exercises the
 * exact same code path the live eval uses.
 */

import { gradeScenario } from "./grader.js";
import type { Scenario, ScenarioResult, ToolCall, ToolDef, ToolResult, Trace } from "./types.js";

/** System prompt mirrors the /portfolio skill's load-bearing rules. */
export const SYSTEM_PROMPT = `You are the Cryptofolio portfolio agent. You manage the user's crypto
portfolio strictly as a data tool: wallets, balances, transactions, and P&L. You do NOT give
trading or tax advice.

Rules:
- Use the provided tools to answer; do not answer balance/value/P&L questions from memory.
- Never fabricate, estimate, or interpolate a balance or price to fill a gap. If data is missing
  or a tool errors, say so plainly.
- When recording a transaction on an account that is already synced from an exchange/chain, set
  cost_basis_only=true so you do not double-count the existing balance.
- Before quoting figures, prefer fresh data; if you cannot get it, report the uncertainty.
Answer concisely.`;

export interface ModelTurn {
  /** Tool calls to execute this step. */
  toolCalls?: ToolCall[];
  /** A final natural-language answer (ends the loop). */
  finalText?: string;
}

export interface ModelContext {
  scenarioId: string;
  /** The user's message for this scenario. */
  prompt: string;
  system: string;
  tools: ToolDef[];
  history: TurnRecord[];
  stepIndex: number;
}

export interface TurnRecord {
  toolCalls: ToolCall[];
  toolResults: ToolResult[];
}

export interface Model {
  step(ctx: ModelContext): Promise<ModelTurn>;
}

export interface Tools {
  list(): Promise<ToolDef[]>;
  call(call: ToolCall): Promise<ToolResult>;
}

export interface RunOptions {
  maxSteps?: number;
}

export async function runScenario(
  scenario: Scenario,
  model: Model,
  tools: Tools,
  opts: RunOptions = {}
): Promise<ScenarioResult> {
  const maxSteps = opts.maxSteps ?? 8;
  const toolDefs = await tools.list();

  const calls: ToolCall[] = [];
  const history: TurnRecord[] = [];
  let finalText = "";
  let error: string | undefined;

  try {
    for (let stepIndex = 0; stepIndex < maxSteps; stepIndex++) {
      const turn = await model.step({
        scenarioId: scenario.id,
        prompt: scenario.prompt,
        system: SYSTEM_PROMPT,
        tools: toolDefs,
        history,
        stepIndex,
      });

      if (turn.finalText !== undefined) {
        finalText = turn.finalText;
        break;
      }

      const toolCalls = turn.toolCalls ?? [];
      if (toolCalls.length === 0) {
        // No tool calls and no final text — treat as a (possibly empty) answer.
        break;
      }

      const toolResults: ToolResult[] = [];
      for (const c of toolCalls) {
        calls.push(c);
        toolResults.push(await tools.call(c));
      }
      history.push({ toolCalls, toolResults });

      if (stepIndex === maxSteps - 1) {
        error = "max steps reached without a final answer";
      }
    }
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  }

  const trace: Trace =
    error === undefined ? { calls, finalText } : { calls, finalText, error };
  return gradeScenario(scenario, trace);
}
