/** Parses what the user typed; `null` while the field is empty or mid-edit (e.g. "-" or "1."). */
export function parseNumber(text: string): number | null {
  if (text.trim() === '') return null;
  const n = Number(text);
  return Number.isFinite(n) ? n : null;
}

export function clamp(n: number, min = -Infinity, max = Infinity): number {
  return Math.min(max, Math.max(min, n));
}
