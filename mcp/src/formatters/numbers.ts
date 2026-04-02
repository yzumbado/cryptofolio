/**
 * Number formatting utilities for LLM-friendly output.
 * All monetary values come from the CLI as Decimal strings — we format them
 * for display but never convert them to JS floats (precision loss).
 */

/**
 * Format a Decimal string as USD with 2 decimal places.
 * e.g. "95000.5" → "$95,000.50"
 */
export function formatUsd(value: string): string {
  const n = parseFloat(value);
  if (isNaN(n)) return value;
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(n);
}

/**
 * Format a Decimal string as a percentage.
 * e.g. "12.345" → "12.35%"
 */
export function formatPct(value: string): string {
  const n = parseFloat(value);
  if (isNaN(n)) return value;
  return `${n.toFixed(2)}%`;
}

/**
 * Format a crypto quantity — preserve up to 8 decimal places, trim trailing zeros.
 * e.g. "0.10000000" → "0.1"
 */
export function formatQuantity(value: string): string {
  const n = parseFloat(value);
  if (isNaN(n)) return value;
  // Use up to 8 decimal places, trim trailing zeros
  return n.toFixed(8).replace(/\.?0+$/, "");
}

/**
 * Sign a Decimal string for display (+ prefix for positive values).
 * e.g. "1234.56" → "+$1,234.56", "-200" → "-$200.00"
 */
export function formatSignedUsd(value: string): string {
  const n = parseFloat(value);
  if (isNaN(n)) return value;
  const formatted = formatUsd(value);
  return n >= 0 ? `+${formatted}` : formatted;
}
