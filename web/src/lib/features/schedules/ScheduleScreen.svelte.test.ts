import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
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

function json(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), { status });
}

beforeEach(() => {
	vi.stubGlobal('fetch', fetchMock);
	fetchMock.mockImplementation(async (url, init) => {
		if (init?.method === 'POST') return json({}, 201);
		return String(url).endsWith('/schedules') ? json([]) : json(job);
	});
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.resetAllMocks();
});

describe('ScheduleScreen', () => {
	it('adds a first schedule from the form', async () => {
		render(ScheduleScreen, { jobId: 4 });

		await expect.element(page.getByText('No schedule yet')).toBeVisible();
		await page.getByRole('button', { name: 'Add schedule' }).click();

		const form = page.getByRole('dialog');
		await form.getByLabelText('Applies to days from').fill('2026-11-02');
		await form
			.getByRole('group', { name: 'Lunch' })
			.getByLabelText('Unpaid', { exact: true })
			.click();
		await form
			.getByRole('group', { name: 'Breaks' })
			.getByLabelText('Paid', { exact: true })
			.click();
		await form.getByRole('textbox', { name: `${monday} work` }).fill('8');
		await form.getByRole('textbox', { name: `${monday} lunch` }).fill('1:00');
		await form.getByRole('button', { name: 'Copy Monday to Tuesday–Friday' }).click();
		await form.getByRole('button', { name: 'Save' }).click();

		await vi.waitFor(() =>
			expect(fetchMock.mock.calls.some(([, init]) => init?.method === 'POST')).toBe(true)
		);
		const [url, init] = fetchMock.mock.calls.find(([, init]) => init?.method === 'POST')!;
		expect(url).toBe('/api/jobs/4/schedules');
		const body = JSON.parse(String(init?.body));
		expect(body).toMatchObject({ validFrom: '2026-11-02', lunchPaid: false, breaksPaid: true });
		expect(body.days.map((day: { weekday: number }) => day.weekday)).toEqual([1, 2, 3, 4, 5]);
		expect(body.days[4]).toEqual({
			weekday: 5,
			workMinutes: 480,
			lunchMinutes: 60,
			breakMinutes: 0
		});
	});

	it('explains what is missing instead of sending an incomplete schedule', async () => {
		render(ScheduleScreen, { jobId: 4 });

		await page.getByRole('button', { name: 'Add schedule' }).click();
		await page.getByRole('dialog').getByRole('button', { name: 'Save' }).click();

		await expect
			.element(page.getByRole('alert'))
			.toHaveTextContent('Choose whether lunch is paid.');
		expect(fetchMock.mock.calls.some(([, init]) => init?.method === 'POST')).toBe(false);
	});
});
