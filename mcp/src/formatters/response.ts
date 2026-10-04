/**
 * Shared response builders for all MCP tool handlers.
 *
 * Every tool returns { content: [{ type: "text", text: string }] }.
 * The text is always JSON so Claude can read it structurally.
 */

import { z } from "zod";
import { CliError } from "../cli.js";
import type { SuccessResponse, ErrorResponse } from "../types.js";

// ---------------------------------------------------------------------------
// Machine-readable output schema
// ---------------------------------------------------------------------------

/**
 * Output schema attached to every tool via `server.registerTool(...)`.
 *
 * The server uses ONE permissive envelope schema rather than a per-tool union:
 * the SDK's `normalizeObjectSchema` returns `undefined` for a `z.union(...)`,
 * and `safeParseAsync(undefined, ...)` then throws, so a union is unsafe here.
 * Every field is optional except `success`, which both envelopes always carry.
 */
export const TOOL_OUTPUT_SCHEMA = z.object({
  success: z
    .boolean()
    .describe("true for a success envelope, false for an error envelope"),
  data: z
    .unknown()
    .optional()
    .describe("Success payload; present when success is true"),
  message: z
    .string()
    .optional()
    .describe("Human-readable summary; present on most successes"),
  error: z
    .string()
    .optional()
    .describe("Error message; present when success is false"),
  code: z
    .string()
    .optional()
    .describe("Machine-readable error code, when the failure has one"),
  hint: z
    .string()
    .optional()
    .describe("Suggested fix for the error, when available"),
});

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

export function buildSuccess<T>(
  data: T,
  message?: string
): SuccessResponse<T> {
  return message !== undefined
    ? { success: true, data, message }
    : { success: true, data };
}

export function buildError(
  error: string,
  code?: string,
  hint?: string
): ErrorResponse {
  const resp: ErrorResponse = { success: false, error };
  if (code !== undefined) resp.code = code;
  if (hint !== undefined) resp.hint = hint;
  return resp;
}

// ---------------------------------------------------------------------------
// Tool content wrapper
// ---------------------------------------------------------------------------

/**
 * Result of wrapping a payload for an MCP tool call.
 *
 * Both fields carry the same object: `content[0].text` is the pretty-printed
 * JSON (unchanged from before), and `structuredContent` is the payload itself
 * for machine consumption. The SDK hard-throws `McpError` when a tool declares
 * an outputSchema but omits `structuredContent`, so every handler return must
 * include both.
 */
export interface ToolContentResult {
  // The SDK's CallToolResult is an inferred Zod object with an index
  // signature, so this alias needs one too to stay assignable.
  [key: string]: unknown;
  content: Array<{ type: "text"; text: string }>;
  structuredContent: Record<string, unknown>;
}

export function toContent(
  payload: SuccessResponse | ErrorResponse
): ToolContentResult {
  return {
    content: [{ type: "text", text: JSON.stringify(payload, null, 2) }],
    structuredContent: { ...payload },
  };
}

// ---------------------------------------------------------------------------
// Tool-description output contract
// ---------------------------------------------------------------------------

/**
 * Sentence appended to every tool description stating the output envelope.
 *
 * Every tool also declares `TOOL_OUTPUT_SCHEMA` as its machine-readable
 * `outputSchema`; this note keeps the same contract visible to the model in
 * prose. `dataShape` is a compact description of the success payload's `data`
 * field.
 */
export function outputEnvelopeNote(dataShape: string): string {
  return (
    `Returns JSON in content[0].text — success: {success: true, data, message} ` +
    `where data is ${dataShape}; failure: {success: false, error, code?, hint?}.`
  );
}

// ---------------------------------------------------------------------------
// Error handler — converts CliError or generic Error to a buildError response
// ---------------------------------------------------------------------------

export function handleCliError(
  err: unknown,
  command: string
): ErrorResponse {
  if (err instanceof CliError) {
    let hint = err.hint;
    if (!hint) {
      if (err.exitCode === 127) {
        hint =
          "Ensure CRYPTOFOLIO_BIN points to the cryptofolio binary, or add it to your PATH.";
      }
    }
    return buildError(
      err.message,
      `CLI_ERROR_${err.exitCode}`,
      hint
    );
  }
  const msg =
    err instanceof Error ? err.message : String(err);
  return buildError(`Unexpected error in ${command}: ${msg}`);
}

// ---------------------------------------------------------------------------
// Pagination helper
// ---------------------------------------------------------------------------

export interface PaginatedResult<T> {
  items: T[];
  total_fetched: number;
  offset: number;
  limit: number;
  has_more: boolean;
  next_offset?: number;
  note?: string;
}

/**
 * Apply server-side pagination to an array.
 * The CLI's --limit fetches `offset + limit` rows; we slice here.
 */
export function paginate<T>(
  all: T[],
  offset: number,
  limit: number
): PaginatedResult<T> {
  const sliced = all.slice(offset, offset + limit);
  const hasMore = offset + limit < all.length;
  const result: PaginatedResult<T> = {
    items: sliced,
    total_fetched: all.length,
    offset,
    limit,
    has_more: hasMore,
  };
  if (hasMore) {
    result.next_offset = offset + limit;
    result.note = `Call again with offset: ${offset + limit} to get the next page.`;
  }
  return result;
}
