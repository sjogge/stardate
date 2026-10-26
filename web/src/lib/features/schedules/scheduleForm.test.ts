import { describe, expect, it } from 'vitest';
import type { Schedule } from '$lib/api/types';
import { copyMondayToWeekdays, draftFrom, toRequest } from './scheduleForm';

const schedule: Schedule = {
	id: 3,
	validFrom: '2026-01-01',
	lunchPaid: false,
	breaksPaid: true,
	days: [
		{ weekday: 1, workMinutes: 480, lunchMinutes: 60, breakMinutes: 30 },
		{ weekday: 5, workMinutes: 300, lunchMinutes: 0, breakMinutes: 15 }
	],
	version: 2,
	createdAt: '2026-01-01T08:00:00Z',
	updatedAt: '2026-01-01T08:00:00Z'
};

describe('scheduleForm', () => {
	it('fills the form from a schedule, with empty days where none is scheduled', () => {
		const draft = draftFrom(schedule, '2026-11-01');

		expect(draft.validFrom).toBe('2026-11-01');
		expect(draft.lunchPaid).toBe(false);
		expect(draft.days[0]).toEqual({ work: '8:00', lunch: '1:00', breaks: '0:30' });
		expect(draft.days[1]).toEqual({ work: '', lunch: '', breaks: '' });
	});

	it('leaves paid lunch and breaks unchosen for a first schedule', () => {
		const draft = draftFrom(undefined, '2026-11-01');

		expect(draft.lunchPaid).toBeUndefined();
		expect(toRequest(draft)).toEqual({ error: 'Choose whether lunch is paid.' });
	});

	it('round-trips a schedule into the same request', () => {
		const result = toRequest(draftFrom(schedule, schedule.validFrom));

		expect(result).toEqual({
			request: {
				validFrom: '2026-01-01',
				lunchPaid: false,
				breaksPaid: true,
				days: schedule.days
			}
		});
	});

	it('copies Monday to Tuesday to Friday only', () => {
		const draft = draftFrom(schedule, '2026-11-01');

		copyMondayToWeekdays(draft);

		expect(draft.days.slice(1, 5)).toEqual(
			Array(4).fill({ work: '8:00', lunch: '1:00', breaks: '0:30' })
		);
		expect(draft.days[5]).toEqual({ work: '', lunch: '', breaks: '' });
	});

	it('names the weekday and field of a duration it cannot read', () => {
		const draft = draftFrom(schedule, '2026-11-01');
		draft.days[2].lunch = '1h';

		const result = toRequest(draft);

		expect(result).toHaveProperty('error');
		expect('error' in result && result.error).toMatch(/lunch: use hours and minutes/);
	});
});
