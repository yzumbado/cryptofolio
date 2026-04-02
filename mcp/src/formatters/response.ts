/**
 * Shared response builders for all MCP tool handlers.
 *
 * Every tool returns { content: [{ type: "text", text: string }] }.
 * The text is always JSON so Claude can read it structurally.
 */

import { CliError } from "../cli.js";
import type { SuccessResponse, ErrorResponse } from "../types.js";

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

export function toContent(
  payload: SuccessResponse | ErrorResponse
): { content: Array<{ type: "text"; text: string }> } {
  return {
    content: [{ type: "text", text: JSON.stringify(payload, null, 2) }],
  };
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
