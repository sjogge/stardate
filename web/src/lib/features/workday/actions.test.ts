import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Day, JobStatus } from '$lib/api/types';
import { actionLabel, chooseActions, runAction } from './actions';

function day(overrides: Partial<Day> = {}): Day {
	return {
		id: 1,
		workDate: '2026-10-26',
		kind: 'work',
		startedAt: '2026-10-26T07:00:00Z',
		endedAt: null,
		note: null,
		scheduleId: 1,
		expected: {
			workMinutes: 480,
			lunchMinutes: 60,
			breakMinutes: 30,
			lunchPaid: false,
			breaksPaid: true
		},
		pauses: [],
		totals: {
			presenceMinutes: 0,
			lunchMinutes: 60,
			breakMinutes: 30,
			otherMinutes: 0,
			allowanceMinutes: 30,
			workedMinutes: 0,
			expectedWorkMinutes: 480,
			balanceMinutes: -480,
			leaveAt: null
		},
		version: 1,
		createdAt: '2026-10-26T07:00:00Z',
		updatedAt: '2026-10-26T07:00:00Z',
		...overrides
	};
}

function status(overrides: Partial<JobStatus>): JobStatus {
	return {
		jobId: 1,
		jobName: 'Acme',
		clockedIn: false,
		runningPause: null,
		availableActions: [],
		suggestedScheduleId: null,
		today: null,
		flexBalanceMinutes: 0,
		...overrides
	};
}

const labels = (choice: ReturnType<typeof chooseActions>) => ({
	primary: choice.primary && actionLabel(choice.primary),
	secondary: choice.secondary.map(actionLabel)
});

describe('chooseActions', () => {
	it('offers clocking in when the backend allows it', () => {
		const choice = chooseActions(status({ availableActions: ['clockIn'] }));

		expect(labels(choice)).toEqual({ primary: 'Clock in', secondary: [] });
	});

	it('puts lunch first while it is expected and not taken', () => {
		const choice = chooseActions(
			status({ clockedIn: true, availableActions: ['startPause', 'clockOut'], today: day() })
		);

		expect(labels(choice)).toEqual({
			primary: 'Start lunch',
			secondary: ['Start break', 'Start absence', 'Clock out']
		});
	});

	it('puts clocking out first once lunch is logged', () => {
		const lunch = { id: 1, kind: 'lunch' as const, startedAt: null, endedAt: null, minutes: 45 };
		const choice = chooseActions(
			status({
				clockedIn: true,
				availableActions: ['startPause', 'clockOut'],
				today: day({ pauses: [lunch] })
			})
		);

		expect(labels(choice)).toEqual({
			primary: 'Clock out',
			secondary: ['Start lunch', 'Start break', 'Start absence']
		});
	});

	it('offers ending the running pause first', () => {
		const choice = chooseActions(
			status({
				clockedIn: true,
				availableActions: ['endPause', 'clockOut'],
				runningPause: { kind: 'break', startedAt: '2026-10-26T09:00:00Z' },
				today: day()
			})
		);

		expect(labels(choice)).toEqual({ primary: 'End break', secondary: ['Clock out'] });
	});

	it('offers nothing when the day is done', () => {
		const choice = chooseActions(status({ availableActions: [], today: day() }));

		expect(labels(choice)).toEqual({ primary: undefined, secondary: [] });
	});
});

describe('runAction', () => {
	const fetchMock = vi.fn<typeof fetch>();

	afterEach(() => {
		vi.unstubAllGlobals();
		vi.resetAllMocks();
	});

	async function sentBody(...args: Parameters<typeof runAction>): Promise<unknown> {
		vi.stubGlobal('fetch', fetchMock);
		fetchMock.mockResolvedValue(new Response(JSON.stringify(day()), { status: 200 }));
		await runAction(...args);
		return JSON.parse(String(fetchMock.mock.calls[0][1]?.body));
	}

	it('clocks in on the local date and schedule, letting the server pick the time', async () => {
		const body = await sentBody(1, { type: 'clockIn' }, { workDate: '2026-10-26' }, 3);

		expect(fetchMock.mock.calls[0][0]).toBe('/api/jobs/1/clock-in');
		expect(body).toEqual({ workDate: '2026-10-26', scheduleId: 3 });
	});

	it('sends a back-dated time with the action', async () => {
		const at = '2026-10-26T06:30:00.000Z';
		const body = await sentBody(
			1,
			{ type: 'startPause', pause: 'lunch' },
			{ workDate: '2026-10-26', at }
		);

		expect(fetchMock.mock.calls[0][0]).toBe('/api/jobs/1/pauses/start');
		expect(body).toEqual({ kind: 'lunch', at });
	});
});
