import { describe, expect, it } from 'vitest';
import type { Day } from '$lib/api/types';
import { draftFromDay, emptyPause, toDayRequest } from './dayForm';

/** An instant at a local date and time, as the device's time zone sees it */
function local(year: number, month: number, day: number, hours: number, minutes = 0): string {
	return new Date(year, month - 1, day, hours, minutes).toISOString();
}

const day: Day = {
	id: 5,
	workDate: '2026-10-20',
	kind: 'work',
	startedAt: local(2026, 10, 20, 8),
	endedAt: local(2026, 10, 20, 17, 30),
	note: 'Release day',
	scheduleId: 1,
	expected: {
		workMinutes: 480,
		lunchMinutes: 60,
		breakMinutes: 30,
		lunchPaid: false,
		breaksPaid: true
	},
	pauses: [
		{
			id: 1,
			kind: 'lunch',
			startedAt: local(2026, 10, 20, 12),
			endedAt: local(2026, 10, 20, 12, 45),
			minutes: null
		},
		{ id: 2, kind: 'break', startedAt: null, endedAt: null, minutes: 15 }
	],
	totals: {
		presenceMinutes: 570,
		lunchMinutes: 45,
		breakMinutes: 15,
		otherMinutes: 0,
		allowanceMinutes: 30,
		workedMinutes: 540,
		expectedWorkMinutes: 480,
		balanceMinutes: 60,
		leaveAt: null
	},
	version: 3,
	createdAt: local(2026, 10, 20, 8),
	updatedAt: local(2026, 10, 20, 17, 30)
};

describe('dayForm', () => {
	it('shows a day in local times and sends it back unchanged', () => {
		const draft = draftFromDay(day, 'ignored');

		expect(draft).toMatchObject({ workDate: '2026-10-20', start: '08:00', end: '17:30' });
		expect(draft.pauses[1]).toEqual({ kind: 'break', start: '', end: '', duration: '0:15' });
		expect(toDayRequest(draft)).toEqual({
			request: {
				kind: 'work',
				startedAt: day.startedAt,
				endedAt: day.endedAt,
				pauses: [
					{ kind: 'lunch', startedAt: day.pauses[0].startedAt, endedAt: day.pauses[0].endedAt },
					{ kind: 'break', minutes: 15 }
				],
				note: 'Release day'
			}
		});
	});

	it('puts an end before the start on the next day', () => {
		const draft = draftFromDay(undefined, '2026-10-23');
		draft.start = '22:00';
		draft.end = '06:00';

		const result = toDayRequest(draft);

		expect(result).toMatchObject({
			request: { startedAt: local(2026, 10, 23, 22), endedAt: local(2026, 10, 24, 6) }
		});
	});

	it('leaves the end out while the day is open', () => {
		const draft = draftFromDay(undefined, '2026-10-26');
		draft.start = '08:00';

		const result = toDayRequest(draft);

		expect(result).toMatchObject({ request: { kind: 'work', startedAt: local(2026, 10, 26, 8) } });
		expect('request' in result && result.request.endedAt).toBeUndefined();
	});

	it('sends a flex day without times or pauses', () => {
		const draft = draftFromDay(day, '2026-10-20');
		draft.kind = 'flex';
		draft.note = '  ';

		expect(toDayRequest(draft)).toEqual({ request: { kind: 'flex', pauses: [], note: null } });
	});

	it('asks for what is missing', () => {
		const draft = draftFromDay(undefined, '2026-10-26');
		expect(toDayRequest(draft)).toEqual({ error: 'Enter the start time.' });

		draft.start = '08:00';
		draft.pauses.push(emptyPause('lunch'));
		expect(toDayRequest(draft)).toEqual({
			error: 'Pause 1 (lunch): enter a start time or a duration.'
		});

		draft.pauses[0].duration = '45 min';
		expect(toDayRequest(draft)).toEqual({
			error: 'Pause 1 (lunch): use hours and minutes, e.g. 0:45.'
		});
	});
});
