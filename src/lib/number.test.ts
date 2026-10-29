import { describe, expect, it } from 'vitest';

import { clamp, parseNumber } from './number';

describe('parseNumber', () => {
  it('treats an empty field as no value instead of 0', () => {
    expect(parseNumber('')).toBeNull();
    expect(parseNumber('  ')).toBeNull();
  });

  it('parses finished numbers and rejects partial input', () => {
    expect(parseNumber('4')).toBe(4);
    expect(parseNumber('0.5')).toBe(0.5);
    expect(parseNumber('-')).toBeNull();
  });
});

describe('clamp', () => {
  it('keeps values inside the range', () => {
    expect(clamp(0, 1, 40)).toBe(1);
    expect(clamp(99, 1, 40)).toBe(40);
    expect(clamp(6, 1, 40)).toBe(6);
  });
});
