/**
 * Unit tests for the display formatters (src/formatters/numbers.ts).
 *
 * The float-corruption cases below are the reason this file exists: the old
 * implementation routed every value through a JS double first, so
 * "9007199254740993.00" rendered as 9007199254740992 and "1.005" as "1.00%".
 * The formatters must now work on the decimal string itself.
 */

import { describe, it, expect } from "vitest";
import { DecimalError } from "../src/decimal.js";
import {
  formatUsd,
  formatPct,
  formatQuantity,
  formatSignedUsd,
} from "../src/formatters/numbers.js";

describe("formatUsd", () => {
  it("formats typical values with 2 decimals and en-US grouping", () => {
    expect(formatUsd("95000.5")).toBe("$95,000.50");
    expect(formatUsd("1234.56")).toBe("$1,234.56");
    expect(formatUsd("-1234.56")).toBe("-$1,234.56");
    expect(formatUsd("1234567.891")).toBe("$1,234,567.89");
    expect(formatUsd("0")).toBe("$0.00");
    expect(formatUsd("100")).toBe("$100.00");
    expect(formatUsd("-200")).toBe("-$200.00");
    expect(formatUsd(".5")).toBe("$0.50");
    expect(formatUsd("+3")).toBe("$3.00");
  });

  it("does not group integer parts shorter than four digits", () => {
    expect(formatUsd("999.99")).toBe("$999.99");
    expect(formatUsd("1000")).toBe("$1,000.00");
  });

  it("renders values past 2^53 exactly instead of collapsing them", () => {
    // The old float path produced $9,007,199,254,740,992.00 here.
    expect(formatUsd("9007199254740993.00")).toBe("$9,007,199,254,740,993.00");
    expect(formatUsd("999999999999999999999")).toBe(
      "$999,999,999,999,999,999,999.00"
    );
    expect(formatUsd("123456789012345678901234567890.12")).toBe(
      "$123,456,789,012,345,678,901,234,567,890.12"
    );
  });

  it("rounds half away from zero exactly", () => {
    expect(formatUsd("0.005")).toBe("$0.01");
    expect(formatUsd("-0.005")).toBe("-$0.01");
    expect(formatUsd("1.0049999999999999")).toBe("$1.00");
  });

  it("keeps the sign of a nonzero negative that rounds to zero", () => {
    expect(formatUsd("-0.000000001")).toBe("-$0.00");
    expect(formatUsd("-0.004")).toBe("-$0.00");
  });

  it("renders a literal zero unsigned", () => {
    expect(formatUsd("-0.00")).toBe("$0.00");
    expect(formatUsd("-0")).toBe("$0.00");
  });
});

describe("formatPct", () => {
  it("formats typical values with 2 decimals and no grouping", () => {
    expect(formatPct("12.345")).toBe("12.35%");
    expect(formatPct("1234567.891")).toBe("1234567.89%");
    expect(formatPct("0")).toBe("0.00%");
    expect(formatPct("-5")).toBe("-5.00%");
    expect(formatPct("95000.5")).toBe("95000.50%");
  });

  it("rounds half away from zero where the float path rounded down", () => {
    // Number(1.005).toFixed(2) === "1.00" and Number(2.675).toFixed(2) === "2.67".
    expect(formatPct("1.005")).toBe("1.01%");
    expect(formatPct("2.675")).toBe("2.68%");
    expect(formatPct("-1.005")).toBe("-1.01%");
  });

  it("renders values past 2^53 exactly instead of in exponent notation", () => {
    expect(formatPct("9007199254740993")).toBe("9007199254740993.00%");
    expect(formatPct("999999999999999999999")).toBe(
      "999999999999999999999.00%"
    );
  });

  it("keeps the sign of a nonzero negative that rounds to zero", () => {
    expect(formatPct("-0.000000001")).toBe("-0.00%");
    expect(formatPct("-0.004")).toBe("-0.00%");
  });

  it("renders a literal zero unsigned", () => {
    expect(formatPct("-0.00")).toBe("0.00%");
    expect(formatPct("-0")).toBe("0.00%");
  });
});

describe("formatQuantity", () => {
  it("shows up to 8 decimals and trims trailing zeros", () => {
    expect(formatQuantity("0.10000000")).toBe("0.1");
    expect(formatQuantity("1.00000000")).toBe("1");
    expect(formatQuantity("0")).toBe("0");
    expect(formatQuantity("100")).toBe("100");
    expect(formatQuantity("-200")).toBe("-200");
    expect(formatQuantity(".5")).toBe("0.5");
    expect(formatQuantity("1234567.891")).toBe("1234567.891");
  });

  it("rounds to 8 decimals half away from zero", () => {
    expect(formatQuantity("0.123456789")).toBe("0.12345679");
    expect(formatQuantity("0.000000005")).toBe("0.00000001");
    expect(formatQuantity("1.123456785")).toBe("1.12345679");
    // The old float path gave -0.12345678 (the double sits below the tie).
    expect(formatQuantity("-0.123456785")).toBe("-0.12345679");
  });

  it("renders values past 2^53 exactly", () => {
    expect(formatQuantity("9007199254740993")).toBe("9007199254740993");
    expect(formatQuantity("999999999999999999999")).toBe("999999999999999999999");
  });

  it("keeps the sign of a nonzero negative that rounds to zero", () => {
    expect(formatQuantity("-0.000000001")).toBe("-0");
    expect(formatQuantity("-0.000000004")).toBe("-0");
  });

  it("renders a literal zero unsigned", () => {
    expect(formatQuantity("-0.00")).toBe("0");
    expect(formatQuantity("-0")).toBe("0");
  });
});

describe("formatSignedUsd", () => {
  it("prefixes a plus sign for non-negative values", () => {
    expect(formatSignedUsd("1234.56")).toBe("+$1,234.56");
    expect(formatSignedUsd("0")).toBe("+$0.00");
    expect(formatSignedUsd("9007199254740993.00")).toBe(
      "+$9,007,199,254,740,993.00"
    );
  });

  it("keeps the minus sign for negative values", () => {
    expect(formatSignedUsd("-200")).toBe("-$200.00");
    expect(formatSignedUsd("-1234.56")).toBe("-$1,234.56");
    expect(formatSignedUsd("-0.000000001")).toBe("-$0.00");
  });

  it("never emits a doubled sign for a literal negative zero", () => {
    // The old float path returned the nonsensical "+-$0.00".
    expect(formatSignedUsd("-0.00")).toBe("+$0.00");
    expect(formatSignedUsd("-0")).toBe("+$0.00");
  });
});

describe("malformed input", () => {
  const malformed = ["", "abc", "12abc", "1e5", "NaN", "Infinity", "1,000", "0x10"];

  it("throws DecimalError from every formatter instead of guessing", () => {
    for (const value of malformed) {
      expect(() => formatUsd(value)).toThrow(DecimalError);
      expect(() => formatPct(value)).toThrow(DecimalError);
      expect(() => formatQuantity(value)).toThrow(DecimalError);
      expect(() => formatSignedUsd(value)).toThrow(DecimalError);
    }
  });

  it("accepts the grammar the CLI can actually emit", () => {
    expect(formatUsd(" 5 ")).toBe("$5.00");
    expect(formatUsd("+3.00")).toBe("$3.00");
    expect(formatQuantity("1.")).toBe("1");
  });
});
