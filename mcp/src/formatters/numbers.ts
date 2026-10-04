/**
 * Number formatting utilities for LLM-friendly output.
 *
 * Every monetary value arrives from the CLI as a Decimal-serialized string (the
 * Rust side prints `Decimal::to_string()`). These formatters operate on the
 * digit string itself through the exact BigInt helpers in ../decimal.ts and
 * never route a value through a JS double, so a balance written as
 * "9007199254740993.00" renders as $9,007,199,254,740,993.00 instead of
 * collapsing to the nearest representable double (2^53). Malformed input throws
 * `DecimalError` rather than silently formatting a parseable prefix such as the
 * "12" in "12abc".
 *
 * Rendering rules, preserved from the earlier floating-point implementation:
 *   - USD and percentages show exactly 2 fractional digits; quantities show up
 *     to 8. All three round half away from zero (the `rust_decimal`
 *     `RoundHalfUp` convention).
 *   - Only USD groups the integer part: en-US 3-digit commas, no grouping for
 *     a 1–3 digit integer part.
 *   - A nonzero negative keeps its "-" even when it rounds to zero (e.g.
 *     "-0.000000001" → "-$0.00"), so the direction of a small loss stays
 *     visible. A literal zero — including "-0.00" — renders unsigned, never as
 *     "-$0.00" or "-0".
 */

import {
  isNegativeDecimalString,
  isZeroDecimalString,
  roundDecimalString,
} from "../decimal.js";

const USD_DECIMAL_PLACES = 2;
const PCT_DECIMAL_PLACES = 2;
const QUANTITY_DECIMAL_PLACES = 8;

/** Whether a value should render with a leading minus (nonzero negative). */
function isNegativeNonzero(value: string): boolean {
  return isNegativeDecimalString(value) && !isZeroDecimalString(value);
}

/** Drop a leading "-" from a rounded decimal string; keep the magnitude. */
function magnitudeOf(rounded: string): string {
  return rounded.startsWith("-") ? rounded.slice(1) : rounded;
}

/** Insert en-US thousands separators into an unsigned decimal string. */
function groupThousands(value: string): string {
  const dot = value.indexOf(".");
  const integerPart = dot === -1 ? value : value.slice(0, dot);
  const fractionPart = dot === -1 ? "" : value.slice(dot);
  return integerPart.replace(/\B(?=(\d{3})+(?!\d))/g, ",") + fractionPart;
}

/**
 * Format a Decimal string as USD with 2 decimal places.
 * e.g. "95000.5" → "$95,000.50"
 */
export function formatUsd(value: string): string {
  const rounded = magnitudeOf(roundDecimalString(value, USD_DECIMAL_PLACES));
  const sign = isNegativeNonzero(value) ? "-" : "";
  return `${sign}$${groupThousands(rounded)}`;
}

/**
 * Format a Decimal string as a percentage.
 * e.g. "12.345" → "12.35%"
 */
export function formatPct(value: string): string {
  const rounded = magnitudeOf(roundDecimalString(value, PCT_DECIMAL_PLACES));
  const sign = isNegativeNonzero(value) ? "-" : "";
  return `${sign}${rounded}%`;
}

/**
 * Format a crypto quantity — preserve up to 8 decimal places, trim trailing zeros.
 * e.g. "0.10000000" → "0.1"
 */
export function formatQuantity(value: string): string {
  const rounded = magnitudeOf(
    roundDecimalString(value, QUANTITY_DECIMAL_PLACES)
  )
    .replace(/0+$/, "")
    .replace(/\.$/, "");
  const sign = isNegativeNonzero(value) ? "-" : "";
  return `${sign}${rounded}`;
}

/**
 * Sign a Decimal string for display (+ prefix for positive values).
 * e.g. "1234.56" → "+$1,234.56", "-200" → "-$200.00"
 */
export function formatSignedUsd(value: string): string {
  const formatted = formatUsd(value);
  return isNegativeNonzero(value) ? formatted : `+${formatted}`;
}
