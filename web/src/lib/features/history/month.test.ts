import { describe, expect, it } from 'vitest';
import { isSameMonth, monthOf, monthRange, shiftMonth } from './month';

describe('month', () => {
	it('reads the month of a date', () => {
		expect(monthOf('2026-10-26')).toEqual({ year: 2026, month: 10 });
	});

	it('shifts across year boundaries', () => {
		expect(shiftMonth({ year: 2026, month: 12 }, 1)).toEqual({ year: 2027, month: 1 });
		expect(shiftMonth({ year: 2026, month: 1 }, -1)).toEqual({ year: 2025, month: 12 });
	});

	it('covers the whole month, including leap days', () => {
		expect(monthRange({ year: 2026, month: 10 })).toEqual({ from: '2026-10-01', to: '2026-10-31' });
		expect(monthRange({ year: 2028, month: 2 })).toEqual({ from: '2028-02-01', to: '2028-02-29' });
	});

	it('compares months', () => {
		expect(isSameMonth({ year: 2026, month: 10 }, { year: 2026, month: 10 })).toBe(true);
		expect(isSameMonth({ year: 2026, month: 10 }, { year: 2025, month: 10 })).toBe(false);
	});
});
