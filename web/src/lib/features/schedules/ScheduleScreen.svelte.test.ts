import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import type { Schedule } from '$lib/api/types';
import { formatWeekday } from '$lib/format';
import ScheduleScreen from './ScheduleScreen.svelte';

const fetchMock = vi.fn<typeof fetch>();
// Weekday labels come from formatWeekday; building them the same way keeps the test in step with it
const monday = formatWeekday(1);

const job = {
	id: 4,
	name: 'Starla Corp',
	archivedAt: null,
	version: 1,
	createdAt: '2026-10-01T08:00:00Z',
	updatedAt: '2026-10-01T08:00:00Z'
};

const daytime: Schedule = {
	id: 7,
	name: 'Daytime',
	lunchPaid: false,
	breaksPaid: true,
	nextScheduleId: null,
	days: [
		{
			weekday: 1,
			start: '08:00',
			end: '17:00',
			lunchMinutes: 60,
			breakMinutes: 30,
			workMinutes: 480
		}
	],
	version: 1,
	createdAt: '2026-10-01T08:00:00Z',
	updatedAt: '2026-10-01T08:00:00Z'
};

let schedules: Schedule[] = [];

function json(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), { status });
}

function sentBody(method: string): Record<string, unknown> {
	const call = fetchMock.mock.calls.find(([, init]) => init?.method === method);
	return JSON.parse(String(call?.[1]?.body));
}

beforeEach(() => {
	schedules = [];
	vi.stubGlobal('fetch', fetchMock);
	fetchMock.mockImplementation(async (url, init) => {
		if (init?.method === 'POST') return json({}, 201);
		return String(url).endsWith('/schedules') ? json(schedules) : json(job);
	});
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.resetAllMocks();
});

describe('ScheduleScreen', () => {
	it('adds a named schedule with planned shifts', async () => {
		render(ScheduleScreen, { jobId: 4 });

		await expect.element(page.getByText('No schedule yet')).toBeVisible();
		await page.getByRole('button', { name: 'Add schedule' }).click();

		const form = page.getByRole('dialog');
		await form.getByLabelText('Name').fill('Nights');
		await form
			.getByRole('group', { name: 'Lunch' })
			.getByLabelText('Unpaid', { exact: true })
			.click();
		await form
			.getByRole('group', { name: 'Breaks' })
			.getByLabelText('Paid', { exact: true })
			.click();
		await form.getByLabelText(`${monday} start`).fill('22:00');
		await form.getByLabelText(`${monday} end`).fill('07:00');
		await form.getByLabelText(`${monday} lunch`).fill('0:30');
		await form.getByRole('button', { name: 'Copy Monday to Tuesday–Friday' }).click();
		await form.getByRole('button', { name: 'Save' }).click();

		await vi.waitFor(() =>
			expect(fetchMock.mock.calls.some(([, init]) => init?.method === 'POST')).toBe(true)
		);
		const body = sentBody('POST');
		expect(body).toMatchObject({ name: 'Nights', lunchPaid: false, nextScheduleId: null });
		expect(body.days).toHaveLength(5);
		expect((body.days as unknown[])[4]).toEqual({
			weekday: 5,
			start: '22:00',
			end: '07:00',
			lunchMinutes: 30,
			breakMinutes: 0
		});
		// Saving closes the dialog once the list has reloaded; waiting for it ends the test cleanly
		await expect.element(form).not.toBeInTheDocument();
	});

	it('links a duplicated week into a rotation', async () => {
		schedules = [daytime];
		render(ScheduleScreen, { jobId: 4 });

		await page.getByRole('button', { name: 'Duplicate Daytime' }).click();
		const form = page.getByRole('dialog');
		await expect.element(form.getByLabelText('Name')).toHaveValue('Daytime (copy)');
		await form.getByLabelText('Name').fill('Evenings');
		await form.getByLabelText('Next week').selectOptions('Daytime');
		await form.getByRole('button', { name: 'Save' }).click();

		await vi.waitFor(() =>
			expect(fetchMock.mock.calls.some(([, init]) => init?.method === 'POST')).toBe(true)
		);
		expect(sentBody('POST')).toMatchObject({ name: 'Evenings', nextScheduleId: 7 });
	});

	it('explains what is missing instead of sending an incomplete schedule', async () => {
		render(ScheduleScreen, { jobId: 4 });

		await page.getByRole('button', { name: 'Add schedule' }).click();
		await page.getByRole('dialog').getByRole('button', { name: 'Save' }).click();

		await expect
			.element(page.getByRole('alert'))
			.toHaveTextContent('Give the schedule a name, e.g. Daytime.');
		expect(fetchMock.mock.calls.some(([, init]) => init?.method === 'POST')).toBe(false);
	});
});
