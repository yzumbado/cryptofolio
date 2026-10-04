/**
 * Exact decimal-string arithmetic.
 *
 * Every monetary value crossing the MCP boundary is a Decimal-serialized
 * string (the Rust CLI prints `Decimal::to_string()`), so doing arithmetic on
 * them with JS numbers silently loses precision — `0.1 + 0.2` is not `0.3` in
 * binary floating point. The helpers below operate on the digit string itself
 * via BigInt and never call `parseFloat`/`Number` on a value.
 *
 * Grammar accepted: an optional sign, digits, and an optional fractional part
 * (`"12"`, `"-0.5"`, `".5"`, `"+3.00"`). Rust's `Decimal` never emits exponent
 * notation, so scientific notation is rejected rather than guessed at — a
 * malformed value throws `DecimalError` instead of silently contributing a
 * wrong (or silently skipped) amount to a total.
 *
 * Rounding is half-up on the magnitude, i.e. ties round away from zero — the
 * same rule as `rust_decimal`'s `RoundingStrategy::RoundHalfUp`.
 */

export class DecimalError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DecimalError";
  }
}

interface ParsedDecimal {
  negative: boolean;
  /** Absolute value, scaled by 10^scale. */
  units: bigint;
  /** Number of fractional digits the value was written with. */
  scale: number;
}

const DECIMAL_PATTERN = /^([+-]?)(\d*)(?:\.(\d*))?$/;

function parseDecimal(value: string): ParsedDecimal {
  const match = DECIMAL_PATTERN.exec(value.trim());
  if (!match) {
    throw new DecimalError(`not a decimal string: ${JSON.stringify(value)}`);
  }
  const sign = match[1] ?? "";
  const intPart = match[2] ?? "";
  const fracPart = match[3] ?? "";
  if (intPart === "" && fracPart === "") {
    throw new DecimalError(`not a decimal string: ${JSON.stringify(value)}`);
  }
  return {
    negative: sign === "-",
    units: BigInt(`${intPart}${fracPart}` || "0"),
    scale: fracPart.length,
  };
}

function formatDecimal(
  negative: boolean,
  units: bigint,
  scale: number
): string {
  // padStart guarantees at least one integer digit.
  const digits = units.toString().padStart(scale + 1, "0");
  const cut = digits.length - scale;
  const intPart = digits.slice(0, cut);
  const fracPart = scale === 0 ? "" : digits.slice(cut);
  const body = scale === 0 ? intPart : `${intPart}.${fracPart}`;
  // Never emit "-0" (or "-0.00") for a zero result.
  return negative && units !== 0n ? `-${body}` : body;
}

/** Scale a parsed value up to `targetScale` (targetScale >= value.scale). */
function toScale(value: ParsedDecimal, targetScale: number): bigint {
  const scaled = value.units * 10n ** BigInt(targetScale - value.scale);
  return value.negative ? -scaled : scaled;
}

/** Round `units / 10^scale` to `decimalPlaces` digits, half away from zero. */
function roundUnits(
  units: bigint,
  scale: number,
  decimalPlaces: number
): bigint {
  if (scale <= decimalPlaces) {
    return units * 10n ** BigInt(decimalPlaces - scale);
  }
  const factor = 10n ** BigInt(scale - decimalPlaces);
  const quotient = units / factor;
  const remainder = units % factor;
  return remainder * 2n >= factor ? quotient + 1n : quotient;
}

function assertDecimalPlaces(decimalPlaces: number): void {
  if (!Number.isInteger(decimalPlaces) || decimalPlaces < 0) {
    throw new DecimalError(
      `decimal places must be a non-negative integer, got ${decimalPlaces}`
    );
  }
}

/**
 * Sum decimal strings exactly, without rounding.
 *
 * The result carries as many fractional digits as the widest input, so
 * `["0.1", "0.2"]` → `"0.3"` and `["1.005", "0.005"]` → `"1.010"`. Round the
 * result explicitly with {@link roundDecimalString} when the caller needs a
 * fixed scale. Throws `DecimalError` on a malformed value rather than
 * dropping it from the total.
 */
export function addDecimalStrings(values: readonly string[]): string {
  if (values.length === 0) return "0";
  const parsed = values.map(parseDecimal);
  let scale = 0;
  for (const value of parsed) {
    if (value.scale > scale) scale = value.scale;
  }
  let sum = 0n;
  for (const value of parsed) {
    sum += toScale(value, scale);
  }
  return formatDecimal(sum < 0n, sum < 0n ? -sum : sum, scale);
}

/**
 * Round a decimal string to `decimalPlaces` fractional digits, half away from
 * zero, padding with zeros when the input is shorter. Always returns exactly
 * `decimalPlaces` fractional digits (e.g. `"0"` at 2 → `"0.00"`).
 */
export function roundDecimalString(
  value: string,
  decimalPlaces: number
): string {
  assertDecimalPlaces(decimalPlaces);
  const parsed = parseDecimal(value);
  return formatDecimal(
    parsed.negative,
    roundUnits(parsed.units, parsed.scale, decimalPlaces),
    decimalPlaces
  );
}

/**
 * Multiply two decimal strings exactly and round the product to
 * `decimalPlaces` half away from zero. Used instead of `a * b` in JS so a rate
 * like `"0.3"` cannot introduce binary-float noise into a recorded quantity.
 */
export function multiplyDecimalStrings(
  left: string,
  right: string,
  decimalPlaces: number
): string {
  assertDecimalPlaces(decimalPlaces);
  const a = parseDecimal(left);
  const b = parseDecimal(right);
  const units = a.units * b.units;
  const scale = a.scale + b.scale;
  return formatDecimal(
    a.negative !== b.negative,
    roundUnits(units, scale, decimalPlaces),
    decimalPlaces
  );
}
