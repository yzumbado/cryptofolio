/**
 * Unit tests for exact decimal-string arithmetic (src/decimal.ts).
 *
 * These exist to prove the MCP layer never routes money through JS floats:
 * every case below is one where `parseFloat`/`Number` gives a different answer.
 */

import { describe, it, expect } from "vitest";
import {
  DecimalError,
  addDecimalStrings,
  roundDecimalString,
  multiplyDecimalStrings,
} from "../src/decimal.js";

describe("addDecimalStrings", () => {
  it("sums 0.1 + 0.2 exactly (float drift case)", () => {
    expect(addDecimalStrings(["0.1", "0.2"])).toBe("0.3");
  });

  it("returns exact results beyond Number.MAX_SAFE_INTEGER", () => {
    // 2^53 + 1 is not representable as a double; parseFloat would give 2^53.
    expect(addDecimalStrings(["9007199254740993", "1"])).toBe(
      "9007199254740994"
    );
  });

  it("keeps full precision on very large fractional values", () => {
    expect(
      addDecimalStrings(["123456789012345678901234567890.12", "0.88"])
    ).toBe("123456789012345678901234567891.00");
  });

  it("handles negative values and mixed signs", () => {
    expect(addDecimalStrings(["-5.5", "2.25"])).toBe("-3.25");
    // Exact zero at the widest input scale, and never a signed zero.
    expect(addDecimalStrings(["-0.1", "0.1"])).toBe("0.0");
    expect(addDecimalStrings(["10", "-3", "-7"])).toBe("0");
  });

  it("preserves trailing zeros at the widest input scale", () => {
    expect(addDecimalStrings(["1.50", "2.50"])).toBe("4.00");
    expect(addDecimalStrings(["1.5", "1.5"])).toBe("3.0");
    expect(addDecimalStrings(["1", "2"])).toBe("3");
  });

  it("accepts explicit plus signs and bare fractional forms", () => {
    expect(addDecimalStrings(["+1.25", ".75"])).toBe("2.00");
  });

  it("returns '0' for an empty list", () => {
    expect(addDecimalStrings([])).toBe("0");
  });

  it("throws DecimalError on a malformed value instead of skipping it", () => {
    expect(() => addDecimalStrings(["1.00", "not-a-number"])).toThrow(
      DecimalError
    );
    expect(() => addDecimalStrings([""])).toThrow(DecimalError);
    expect(() => addDecimalStrings(["1e5"])).toThrow(DecimalError);
  });
});

describe("roundDecimalString", () => {
  it("rounds half away from zero, where floats round the wrong way", () => {
    // Number(1.005).toFixed(2) === "1.00"; Number(2.675).toFixed(2) === "2.67".
    expect(roundDecimalString("1.005", 2)).toBe("1.01");
    expect(roundDecimalString("2.675", 2)).toBe("2.68");
    expect(roundDecimalString("-1.005", 2)).toBe("-1.01");
  });

  it("pads to exactly the requested number of decimal places", () => {
    expect(roundDecimalString("0", 2)).toBe("0.00");
    expect(roundDecimalString("1.2", 8)).toBe("1.20000000");
    expect(roundDecimalString("5", 0)).toBe("5");
  });

  it("rounds half-up at scale 0 and truncates below the round point", () => {
    expect(roundDecimalString("1.5", 0)).toBe("2");
    expect(roundDecimalString("-1.5", 0)).toBe("-2");
    expect(roundDecimalString("1.4999", 2)).toBe("1.50");
  });

  it("never emits a signed zero", () => {
    expect(roundDecimalString("-0.001", 2)).toBe("0.00");
  });

  it("rejects a negative or fractional decimal-place count", () => {
    expect(() => roundDecimalString("1", -1)).toThrow(DecimalError);
    expect(() => roundDecimalString("1", 1.5)).toThrow(DecimalError);
  });
});

describe("multiplyDecimalStrings", () => {
  it("multiplies exactly at 8dp (float-noise case)", () => {
    // (100000000 * 1.1).toFixed(8) === "110000000.00000001" with floats.
    expect(multiplyDecimalStrings("100000000", "1.1", 8)).toBe(
      "110000000.00000000"
    );
    expect(multiplyDecimalStrings("123456789", "1.1", 8)).toBe(
      "135802467.90000000"
    );
  });

  it("returns a fixed 8dp string for the brief's track_conversion example", () => {
    // Note: toFixed(8) already hides the drift for this small pair; the
    // large-magnitude cases above are the ones that actually differ.
    expect(multiplyDecimalStrings("0.1", "0.3", 8)).toBe("0.03000000");
    expect(multiplyDecimalStrings("0.1", "0.3", 8)).not.toContain(
      "0.030000000000000002"
    );
  });

  it("handles rates below 1 and exact products", () => {
    expect(multiplyDecimalStrings("500000", "0.001", 8)).toBe("500.00000000");
    expect(multiplyDecimalStrings("500", "0.0000153", 8)).toBe("0.00765000");
  });

  it("rounds the product half away from zero", () => {
    expect(multiplyDecimalStrings("1.005", "1", 2)).toBe("1.01");
  });

  it("tracks the product sign and collapses to unsigned zero", () => {
    expect(multiplyDecimalStrings("-2", "3", 8)).toBe("-6.00000000");
    expect(multiplyDecimalStrings("-2", "-3", 8)).toBe("6.00000000");
    expect(multiplyDecimalStrings("0", "-3", 8)).toBe("0.00000000");
  });

  it("rounds a sub-precision product down to zero", () => {
    // 1e-16 is far below the 8dp rounding point.
    expect(multiplyDecimalStrings("0.00000001", "0.00000001", 8)).toBe(
      "0.00000000"
    );
  });

  it("throws DecimalError on malformed operands", () => {
    expect(() => multiplyDecimalStrings("abc", "1", 8)).toThrow(DecimalError);
    expect(() => multiplyDecimalStrings("1", "", 8)).toThrow(DecimalError);
  });
});
