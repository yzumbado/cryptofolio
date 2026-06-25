/**
 * Live wiring for the eval harness — the keyed path.
 *
 * - RealModel: drives a real Claude model via the Anthropic Messages API,
 *   reconstructing the tool-use conversation from the harness history each step.
 * - McpTools: talks to the REAL MCP server over stdio (exactly as Claude Desktop
 *   does), so the tool descriptions/schemas/handlers under test are the real ones.
 * - makeTempEnv: isolates each run in a throwaway config dir so the seeded DB
 *   never touches the user's data.
 *
 * Requires: ANTHROPIC_API_KEY, and CRYPTOFOLIO_BIN pointing at a built binary.
 * This module is only imported by run.ts when NOT in --mock mode.
 */

import Anthropic from "@anthropic-ai/sdk";
import { execa } from "execa";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

import type { Model, ModelContext, ModelTurn, Tools } from "./harness.js";
import type { ToolCall, ToolDef, ToolResult } from "./types.js";

export const DEFAULT_MODEL = process.env.EVAL_MODEL ?? "claude-sonnet-4-6";

// ---------------------------------------------------------------------------
// Isolated environment for a run
// ---------------------------------------------------------------------------

export interface EvalEnv {
  /** Env vars to pass to the binary + MCP server so they share an isolated DB. */
  env: Record<string, string>;
  bin: string;
  dir: string;
}

export function makeTempEnv(): EvalEnv {
  const bin = process.env.CRYPTOFOLIO_BIN;
  if (!bin) {
    throw new Error("CRYPTOFOLIO_BIN must point at a built cryptofolio binary for live evals");
  }
  const dir = mkdtempSync(join(tmpdir(), "cryptofolio-eval-"));
  // Isolate config/DB: the CLI resolves its DB under the config home.
  const env: Record<string, string> = {
    ...stringEnv(process.env),
    HOME: dir,
    XDG_CONFIG_HOME: join(dir, ".config"),
    XDG_DATA_HOME: join(dir, ".local", "share"),
    CRYPTOFOLIO_BIN: bin,
  };
  return { env, bin, dir };
}

function stringEnv(env: NodeJS.ProcessEnv): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(env)) {
    if (typeof v === "string") out[k] = v;
  }
  return out;
}

/** Run the scenario's setup CLI commands against the isolated DB. */
export async function runSetup(evalEnv: EvalEnv, setup: string[][]): Promise<void> {
  for (const args of setup) {
    await execa(evalEnv.bin, [...args, "--quiet"], { env: evalEnv.env, reject: false });
  }
}

// ---------------------------------------------------------------------------
// Tools — real MCP server over stdio
// ---------------------------------------------------------------------------

export class McpTools implements Tools {
  private client: Client;
  private connected = false;

  constructor(
    private serverEntry: string,
    private env: Record<string, string>
  ) {
    this.client = new Client({ name: "cryptofolio-eval", version: "0.0.1" });
  }

  private async ensure(): Promise<void> {
    if (this.connected) return;
    const transport = new StdioClientTransport({
      command: process.execPath, // node
      args: [this.serverEntry],
      env: this.env,
    });
    await this.client.connect(transport);
    this.connected = true;
  }

  async list(): Promise<ToolDef[]> {
    await this.ensure();
    const res = await this.client.listTools();
    return res.tools.map((t) => ({
      name: t.name,
      description: t.description ?? "",
      inputSchema: (t.inputSchema ?? {}) as Record<string, unknown>,
    }));
  }

  async call(call: ToolCall): Promise<ToolResult> {
    await this.ensure();
    const res = await this.client.callTool({ name: call.name, arguments: call.input });
    const blocks = (res.content ?? []) as Array<{ type: string; text?: string }>;
    const output = blocks
      .filter((b) => b.type === "text" && typeof b.text === "string")
      .map((b) => b.text as string)
      .join("\n");
    return { name: call.name, output, isError: res.isError === true };
  }

  async close(): Promise<void> {
    if (this.connected) await this.client.close();
  }
}

// ---------------------------------------------------------------------------
// Model — real Anthropic Messages API
// ---------------------------------------------------------------------------

export class RealModel implements Model {
  private client: Anthropic;

  constructor(private model: string = DEFAULT_MODEL) {
    this.client = new Anthropic();
  }

  async step(ctx: ModelContext): Promise<ModelTurn> {
    const tools: Anthropic.Tool[] = ctx.tools.map((t) => ({
      name: t.name,
      description: t.description,
      input_schema: t.inputSchema as Anthropic.Tool.InputSchema,
    }));

    const resp = await this.client.messages.create({
      model: this.model,
      max_tokens: 1024,
      system: ctx.system,
      tools,
      messages: buildMessages(ctx),
    });

    const toolCalls: ToolCall[] = [];
    let text = "";
    for (const block of resp.content) {
      if (block.type === "tool_use") {
        toolCalls.push({ name: block.name, input: (block.input ?? {}) as Record<string, unknown> });
      } else if (block.type === "text") {
        text += block.text;
      }
    }

    if (resp.stop_reason === "tool_use" && toolCalls.length > 0) {
      return { toolCalls };
    }
    return { finalText: text };
  }
}

function buildMessages(ctx: ModelContext): Anthropic.MessageParam[] {
  const messages: Anthropic.MessageParam[] = [{ role: "user", content: ctx.prompt }];

  ctx.history.forEach((rec, hi) => {
    const assistantBlocks: Anthropic.ContentBlockParam[] = rec.toolCalls.map((c, ci) => ({
      type: "tool_use",
      id: `call_${hi}_${ci}`,
      name: c.name,
      input: c.input,
    }));
    messages.push({ role: "assistant", content: assistantBlocks });

    const resultBlocks: Anthropic.ContentBlockParam[] = rec.toolResults.map((r, ci) => ({
      type: "tool_result",
      tool_use_id: `call_${hi}_${ci}`,
      content: r.output,
      is_error: r.isError,
    }));
    messages.push({ role: "user", content: resultBlocks });
  });

  return messages;
}
