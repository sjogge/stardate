import { describe, expect, it } from 'vitest';
import type { Schedule } from '$lib/api/types';
import { formatWeekday } from '$lib/format';
import { copyMondayToWeekdays, draftFrom, duplicateDraft, toRequest } from './scheduleForm';

const nights: Schedule = {
	id: 3,
	name: 'Nights',
	lunchPaid: false,
	breaksPaid: true,
	nextScheduleId: 1,
	days: [
		{
			weekday: 1,
			start: '22:00',
			end: '07:00',
			lunchMinutes: 30,
			breakMinutes: 30,
			workMinutes: 510
		},
		{
			weekday: 5,
			start: '22:00',
			end: '06:00',
			lunchMinutes: 0,
			breakMinutes: 15,
			workMinutes: 480
		}
	],
	version: 2,
	createdAt: '2026-01-01T08:00:00Z',
	updatedAt: '2026-01-01T08:00:00Z'
};

describe('scheduleForm', () => {
	it('fills the form from a schedule, with empty days where none is planned', () => {
		const draft = draftFrom(nights);

		expect(draft).toMatchObject({ name: 'Nights', lunchPaid: false, nextScheduleId: 1 });
		expect(draft.days[0]).toEqual({ start: '22:00', end: '07:00', lunch: '0:30', breaks: '0:30' });
		expect(draft.days[1]).toEqual({ start: '', end: '', lunch: '', breaks: '' });
	});

	it('round-trips a schedule into the same request, without the derived work', () => {
		expect(toRequest(draftFrom(nights))).toEqual({
			request: {
				name: 'Nights',
				lunchPaid: false,
				breaksPaid: true,
				nextScheduleId: 1,
				days: [
					{ weekday: 1, start: '22:00', end: '07:00', lunchMinutes: 30, breakMinutes: 30 },
					{ weekday: 5, start: '22:00', end: '06:00', lunchMinutes: 0, breakMinutes: 15 }
				]
			}
		});
	});

	it('duplicates a schedule under a new name and without its rotation link', () => {
		const draft = duplicateDraft(nights);

		expect(draft.name).toBe('Nights (copy)');
		expect(draft.nextScheduleId).toBeNull();
		expect(draft.days[0].start).toBe('22:00');
	});

	it('asks for a name and the paid choices of a new schedule', () => {
		const draft = draftFrom(undefined);
		expect(toRequest(draft)).toEqual({ error: 'Give the schedule a name, e.g. Daytime.' });

		draft.name = 'Daytime';
		expect(toRequest(draft)).toEqual({ error: 'Choose whether lunch is paid.' });
	});

	it('copies Monday to Tuesday to Friday only', () => {
		const draft = draftFrom(nights);

		copyMondayToWeekdays(draft);

		expect(draft.days.slice(1, 5)).toEqual(
			Array(4).fill({ start: '22:00', end: '07:00', lunch: '0:30', breaks: '0:30' })
		);
		expect(draft.days[5]).toEqual({ start: '', end: '', lunch: '', breaks: '' });
	});

	it('names the weekday of a shift it cannot send', () => {
		const draft = draftFrom(nights);
		draft.days[2].start = '09:00';
		expect(toRequest(draft)).toEqual({
			error: `${formatWeekday(3)}: enter both start and end, or clear the day.`
		});

		draft.days[2].end = '09:00';
		expect(toRequest(draft)).toEqual({ error: `${formatWeekday(3)}: start and end must differ.` });

		draft.days[2].end = '17:00';
		draft.days[2].lunch = '1h';
		expect(toRequest(draft)).toEqual({
			error: `${formatWeekday(3)}: enter lunch and breaks in hours and minutes, e.g. 0:30.`
		});
	});
});
