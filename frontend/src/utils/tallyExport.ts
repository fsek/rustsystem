import type { TallyResult } from "@/signatures/voteSession";

// ─── JSON ─────────────────────────────────────────────────────────────────────

export function tallyToJson(tally: TallyResult): string {
  return JSON.stringify(tally, null, 2);
}

// ─── YAML ─────────────────────────────────────────────────────────────────────

function yamlQuote(s: string): string {
  if (
    s === "" ||
    /[:#\[\]{}|>&*!,?@`'"\\]/.test(s) ||
    s.trim() !== s ||
    /^(true|false|null|~|yes|no|on|off)$/i.test(s) ||
    /^[-+]?\d/.test(s)
  ) {
    return JSON.stringify(s);
  }
  return s;
}

export function tallyToYaml(tally: TallyResult): string {
  const lines = ["score:"];
  for (const [k, v] of Object.entries(tally.score)) {
    lines.push(`  ${yamlQuote(k)}: ${v}`);
  }
  lines.push(`blank: ${tally.blank}`);
  return lines.join("\n") + "\n";
}

// ─── TOML ─────────────────────────────────────────────────────────────────────

function tomlKey(s: string): string {
  return /^[A-Za-z0-9_-]+$/.test(s) ? s : JSON.stringify(s);
}

export function tallyToToml(tally: TallyResult): string {
  const lines = [`blank = ${tally.blank}`, "", "[score]"];
  for (const [k, v] of Object.entries(tally.score)) {
    lines.push(`${tomlKey(k)} = ${v}`);
  }
  return lines.join("\n") + "\n";
}

// ─── RON ──────────────────────────────────────────────────────────────────────

export function tallyToRon(tally: TallyResult): string {
  const entries = Object.entries(tally.score)
    .map(([k, v]) => `        ${JSON.stringify(k)}: ${v}`)
    .join(",\n");
  return `TallyResult(\n    score: {\n${entries},\n    },\n    blank: ${tally.blank},\n)\n`;
}

// ─── BSON ─────────────────────────────────────────────────────────────────────

function concatU8(arrays: Uint8Array[]): Uint8Array {
  const len = arrays.reduce((s, a) => s + a.length, 0);
  const out = new Uint8Array(len);
  let off = 0;
  for (const a of arrays) {
    out.set(a, off);
    off += a.length;
  }
  return out;
}

function bsonInt32(n: number): Uint8Array {
  const b = new Uint8Array(4);
  new DataView(b.buffer).setInt32(0, n, true);
  return b;
}

function bsonFloat64(n: number): Uint8Array {
  const b = new Uint8Array(8);
  new DataView(b.buffer).setFloat64(0, n, true);
  return b;
}

function bsonCString(s: string): Uint8Array {
  return concatU8([new TextEncoder().encode(s), new Uint8Array([0])]);
}

function bsonDoc(
  entries: Array<[string, number | Record<string, number>]>,
): Uint8Array {
  const elems: Uint8Array[] = [];
  for (const [key, value] of entries) {
    const k = bsonCString(key);
    if (typeof value === "object") {
      elems.push(
        concatU8([new Uint8Array([0x03]), k, bsonDoc(Object.entries(value))]),
      );
    } else if (
      Number.isInteger(value) &&
      value >= -2147483648 &&
      value <= 2147483647
    ) {
      elems.push(concatU8([new Uint8Array([0x10]), k, bsonInt32(value)]));
    } else {
      elems.push(concatU8([new Uint8Array([0x01]), k, bsonFloat64(value)]));
    }
  }
  const body = concatU8([...elems, new Uint8Array([0x00])]);
  return concatU8([bsonInt32(4 + body.length), body]);
}

export function tallyToBson(tally: TallyResult): ArrayBuffer {
  return bsonDoc([
    ["score", tally.score],
    ["blank", tally.blank],
  ]).buffer;
}
