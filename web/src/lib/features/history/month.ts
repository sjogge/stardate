import { localIsoDate } from '$lib/format';

/** A calendar month; `month` is 1 for January */
export interface Month {
	year: number;
	month: number;
}

/** The month a `YYYY-MM-DD` date falls in */
export function monthOf(isoDate: string): Month {
	const [year, month] = isoDate.split('-').map(Number);
	return { year, month };
}

/** The month `by` months later (negative: earlier) */
export function shiftMonth({ year, month }: Month, by: number): Month {
	// Date normalizes overflowing months, e.g. month 13 of 2026 is January 2027
	const shifted = new Date(year, month - 1 + by, 1);
	return { year: shifted.getFullYear(), month: shifted.getMonth() + 1 };
}

/** The first and last day of the month, inclusive, as the API's `from` and `to` */
export function monthRange({ year, month }: Month): { from: string; to: string } {
	return {
		from: localIsoDate(new Date(year, month - 1, 1)),
		// Day 0 of the next month is the last day of this one
		to: localIsoDate(new Date(year, month, 0))
	};
}

export function isSameMonth(a: Month, b: Month): boolean {
	return a.year === b.year && a.month === b.month;
}
