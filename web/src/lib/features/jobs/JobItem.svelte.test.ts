import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import type { Job } from '$lib/api/types';
import JobItem from './JobItem.svelte';
import { Jobs } from './jobs.svelte';

const fetchMock = vi.fn<typeof fetch>();

function job(overrides: Partial<Job> = {}): Job {
	return {
		id: 7,
		name: 'Starla Corp',
		archivedAt: null,
		version: 3,
		createdAt: '2026-10-01T08:00:00Z',
		updatedAt: '2026-10-01T08:00:00Z',
		...overrides
	};
}

function requests(): string[] {
	return fetchMock.mock.calls.map(([url, init]) => `${init?.method} ${url}`);
}

beforeEach(() => {
	vi.stubGlobal('fetch', fetchMock);
	// Every request answers with an empty job list; the tests only look at what was sent
	fetchMock.mockImplementation(async () => new Response('[]', { status: 200 }));
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.resetAllMocks();
});

describe('JobItem', () => {
	it('archives an active job only after confirming', async () => {
		render(JobItem, { job: job(), jobs: new Jobs() });

		await page.getByRole('button', { name: 'Archive Starla Corp' }).click();
		await expect.element(page.getByRole('alertdialog')).toBeVisible();
		expect(requests()).toEqual([]);

		await page.getByRole('button', { name: 'Archive', exact: true }).click();

		await vi.waitFor(() => expect(requests()).toContain('POST /api/jobs/7/archive'));
	});

	it('sends nothing when the confirmation is cancelled', async () => {
		render(JobItem, { job: job(), jobs: new Jobs() });

		await page.getByRole('button', { name: 'Archive Starla Corp' }).click();
		await page.getByRole('button', { name: 'Cancel' }).click();

		await expect.element(page.getByRole('alertdialog')).not.toBeInTheDocument();
		expect(requests()).toEqual([]);
	});

	it('deletes an archived job with its version', async () => {
		render(JobItem, { job: job({ archivedAt: '2026-10-20T08:00:00Z' }), jobs: new Jobs() });

		await page.getByRole('button', { name: 'Delete Starla Corp' }).click();
		await page.getByRole('button', { name: 'Delete', exact: true }).click();

		await vi.waitFor(() => expect(requests()).toContain('DELETE /api/jobs/7'));
		const deleteCall = fetchMock.mock.calls.find(([, init]) => init?.method === 'DELETE');
		expect(new Headers(deleteCall?.[1]?.headers).get('If-Match')).toBe('"3"');
	});

	it('renames with the trimmed name and the current version', async () => {
		render(JobItem, { job: job(), jobs: new Jobs() });

		await page.getByRole('button', { name: 'Rename Starla Corp' }).click();
		await page.getByRole('textbox', { name: 'Job name' }).fill('  Starla AB ');
		await page.getByRole('button', { name: 'Save' }).click();

		await vi.waitFor(() => expect(requests()).toContain('PUT /api/jobs/7'));
		const putCall = fetchMock.mock.calls.find(([, init]) => init?.method === 'PUT');
		expect(putCall?.[1]?.body).toBe('{"name":"Starla AB"}');
		expect(new Headers(putCall?.[1]?.headers).get('If-Match')).toBe('"3"');
	});
});
