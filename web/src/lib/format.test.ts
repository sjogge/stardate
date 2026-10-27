import { describe, expect, it } from 'vitest';
import {
	formatDate,
	formatBalance,
	formatDuration,
	formatShortDate,
	formatTime,
	localIsoDate,
	formatWeekday,
	localTimeOfDay,
	parseDuration,
	textLocale,
	toInstant
} from './format';

describe('format', () => {
	it('formats a local date as YYYY-MM-DD', () => {
		expect(localIsoDate(new Date(2026, 0, 5, 23, 59))).toBe('2026-01-05');
	});

	it('formats minutes as hours and minutes', () => {
		expect(formatDuration(425)).toBe('7:05');
		expect(formatDuration(0)).toBe('0:00');
		expect(formatDuration(-30)).toBe('-0:30');
	});

	it('signs a balance', () => {
		expect(formatBalance(15)).toBe('+0:15');
		expect(formatBalance(0)).toBe('0:00');
		expect(formatBalance(-90)).toBe('-1:30');
	});

	it('formats an instant as a local clock time', () => {
		expect(formatTime('2026-10-26T07:02:00Z', 'sv-SE', 'Europe/Stockholm')).toBe('08:02');
	});

	it('formats a local time of day as HH:MM', () => {
		expect(localTimeOfDay(new Date(2026, 0, 5, 8, 5))).toBe('08:05');
	});

	it('reads a local date and time as an instant in the device time zone', () => {
		const instant = toInstant({ date: '2026-10-26', time: '08:05' });

		expect(instant).toBe(new Date(2026, 9, 26, 8, 5).toISOString());
	});

	it('gives no instant for an incomplete date or time', () => {
		expect(toInstant({ date: '', time: '08:05' })).toBeUndefined();
		expect(toInstant({ date: '2026-10-26', time: '' })).toBeUndefined();
	});

	it('formats a date with its weekday, on the local calendar day', () => {
		expect(formatShortDate('2026-10-25', 'en-GB')).toBe('Sun 25 Oct');
	});

	it('formats a date with its year', () => {
		expect(formatDate('2026-10-26', 'en-GB')).toBe('26 Oct 2026');
	});

	it('names ISO weekdays from Monday to Sunday', () => {
		expect(formatWeekday(1, 'en-GB')).toBe('Mon');
		expect(formatWeekday(7, 'en-GB')).toBe('Sun');
	});

	it('reads typed durations as minutes', () => {
		expect(parseDuration('8')).toBe(480);
		expect(parseDuration(' 7:30 ')).toBe(450);
		expect(parseDuration('0:45')).toBe(45);
		expect(parseDuration('')).toBe(0);
		expect(parseDuration('24:00')).toBe(1440);
	});

	it('rejects text that is not a duration of at most a day', () => {
		expect(parseDuration('7.5')).toBeUndefined();
		expect(parseDuration('8:60')).toBeUndefined();
		expect(parseDuration('24:01')).toBeUndefined();
		expect(parseDuration('-1')).toBeUndefined();
	});

	it('reads longer durations up to a given maximum', () => {
		expect(parseDuration('40:00', 100 * 60)).toBe(2400);
		expect(parseDuration('40:00')).toBeUndefined();
	});

	it('writes dates in English, whatever the device locale', () => {
		expect(textLocale()).toMatch(/^en/);
	});
});
