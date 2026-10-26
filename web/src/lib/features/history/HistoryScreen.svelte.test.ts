import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import type { Day } from '$lib/api/types';
import { formatMonth, formatShortDate, todayIsoDate } from '$lib/format';
import HistoryScreen from './HistoryScreen.svelte';
import { monthOf, monthRange, shiftMonth } from './month';

const fetchMock = vi.fn<typeof fetch>();
const thisMonth = monthOf(todayIsoDate());
const { from } = monthRange(thisMonth);
// Two days of this month (the API is mocked, so they need not be in the past)
const secondDay = from.replace(/-01$/, '-02');

function day(workDate: string, balanceMinutes: number): Day {
	return {
		id: Number(workDate.slice(-2)),
		workDate,
		kind: 'work',
		startedAt: `${workDate}T07:00:00Z`,
		endedAt: `${workDate}T16:00:00Z`,
		note: workDate === from ? 'Forgot to clock in' : null,
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
			presenceMinutes: 540,
			lunchMinutes: 60,
			breakMinutes: 30,
			otherMinutes: 0,
			allowanceMinutes: 30,
			workedMinutes: 480 + balanceMinutes,
			expectedWorkMinutes: 480,
			balanceMinutes,
			leaveAt: null
		},
		version: 1,
		createdAt: `${workDate}T07:00:00Z`,
		updatedAt: `${workDate}T16:00:00Z`
	};
}

const totals = {
	workDays: 2,
	flexDays: 0,
	workedMinutes: 975,
	expectedWorkMinutes: 960,
	daysBalanceMinutes: 15,
	adjustmentsMinutes: 0,
	balanceMinutes: 15
};

function json(body: unknown): Response {
	return new Response(JSON.stringify(body), { status: 200 });
}

function requested(path: string): boolean {
	return fetchMock.mock.calls.some(([url]) => String(url).startsWith(path));
}

beforeEach(() => {
	vi.stubGlobal('fetch', fetchMock);
	fetchMock.mockImplementation(async (url, init) => {
		const path = String(url);
		if (init?.method === 'PUT') return json({});
		if (init?.method === 'DELETE') return new Response(null, { status: 204 });
		if (path === '/api/jobs') {
			return json([
				{ id: 1, name: 'Starla Corp', archivedAt: null, version: 1, createdAt: '', updatedAt: '' }
			]);
		}
		if (path.includes('/summary')) return json({ from, to: from, totals });
		// Only this month has days; earlier months are empty
		return json(path.includes(`from=${from}`) ? [day(from, -30), day(secondDay, 45)] : []);
	});
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.resetAllMocks();
});

describe('HistoryScreen', () => {
	it("shows this month's days, newest first, with the month's totals", async () => {
		render(HistoryScreen);

		await expect
			.element(page.getByRole('heading', { name: formatMonth(thisMonth.year, thisMonth.month) }))
			.toBeVisible();
		await expect.element(page.getByLabelText(/^Totals for/)).toHaveTextContent('+0:15');

		const rows = page.getByRole('table').getByRole('rowheader');
		await expect.element(rows.first()).toHaveTextContent(formatShortDate(secondDay));
		await expect.element(rows.last()).toHaveTextContent(formatShortDate(from));
		await expect.element(page.getByRole('table')).toHaveTextContent('Forgot to clock in');
		await expect.element(page.getByRole('button', { name: 'Next month' })).toBeDisabled();
	});

	it('loads the previous month when paging back', async () => {
		render(HistoryScreen);
		await expect.element(page.getByRole('table')).toBeVisible();

		await page.getByRole('button', { name: 'Previous month' }).click();

		const previous = shiftMonth(thisMonth, -1);
		const range = monthRange(previous);
		await expect
			.element(
				page.getByText(`No days registered in ${formatMonth(previous.year, previous.month)}.`)
			)
			.toBeVisible();
		expect(requested(`/api/jobs/1/days?from=${range.from}&to=${range.to}`)).toBe(true);
		await expect.element(page.getByRole('button', { name: 'Next month' })).toBeEnabled();
	});
});

describe('editing days', () => {
	function sent(method: string): [string, RequestInit] {
		const call = fetchMock.mock.calls.find(([, init]) => init?.method === method);
		expect(call).toBeDefined();
		return [String(call![0]), call![1]!];
	}

	async function waitFor(method: string): Promise<void> {
		await vi.waitFor(() =>
			expect(fetchMock.mock.calls.some(([, init]) => init?.method === method)).toBe(true)
		);
	}

	it('adds a day that may only be created, not overwritten', async () => {
		render(HistoryScreen);
		const date = from.replace(/-01$/, '-15');

		await page.getByRole('button', { name: 'Add day' }).click();
		const form = page.getByRole('dialog');
		await form.getByLabelText('Date').fill(date);
		await form.getByLabelText('Start').fill('08:00');
		await form.getByLabelText('End', { exact: true }).fill('16:30');
		await form.getByRole('button', { name: 'Add pause' }).click();
		await form.getByLabelText('Pause 1 length').fill('0:45');
		await form.getByRole('button', { name: 'Save' }).click();

		await waitFor('PUT');
		const [url, init] = sent('PUT');
		expect(url).toBe(`/api/jobs/1/days/${date}`);
		expect(new Headers(init.headers).get('If-None-Match')).toBe('*');
		expect(JSON.parse(String(init.body))).toMatchObject({
			kind: 'work',
			pauses: [{ kind: 'lunch', minutes: 45 }],
			note: null
		});
		await expect.element(form).not.toBeInTheDocument();
	});

	it('explains that a date is taken instead of sending it', async () => {
		render(HistoryScreen);

		await page.getByRole('button', { name: 'Add day' }).click();
		const form = page.getByRole('dialog');
		await form.getByLabelText('Date').fill(secondDay);
		await form.getByLabelText('Start').fill('08:00');
		await form.getByRole('button', { name: 'Save' }).click();

		await expect.element(form.getByRole('alert')).toHaveTextContent('already registered');
		expect(fetchMock.mock.calls.some(([, init]) => init?.method === 'PUT')).toBe(false);
	});

	it('saves an edited day with its version', async () => {
		render(HistoryScreen);

		// The table's button: the phone cards with the same buttons are only hidden by CSS
		await page
			.getByRole('table')
			.getByRole('button', { name: `Edit ${formatShortDate(secondDay)}` })
			.click();
		const form = page.getByRole('dialog');
		await form.getByLabelText('Note').fill('Left early');
		await form.getByRole('button', { name: 'Save' }).click();

		await waitFor('PUT');
		const [url, init] = sent('PUT');
		expect(url).toBe(`/api/jobs/1/days/${secondDay}`);
		expect(new Headers(init.headers).get('If-Match')).toBe('"1"');
		expect(JSON.parse(String(init.body)).note).toBe('Left early');
	});

	it('deletes a day after confirming', async () => {
		render(HistoryScreen);

		// The table's button: the phone cards with the same buttons are only hidden by CSS
		await page
			.getByRole('table')
			.getByRole('button', { name: `Edit ${formatShortDate(secondDay)}` })
			.click();
		await page.getByRole('dialog').getByRole('button', { name: 'Delete' }).click();
		await page.getByRole('alertdialog').getByRole('button', { name: 'Delete' }).click();

		await waitFor('DELETE');
		const [url, init] = sent('DELETE');
		expect(url).toBe(`/api/jobs/1/days/${secondDay}`);
		expect(new Headers(init.headers).get('If-Match')).toBe('"1"');
	});
});
