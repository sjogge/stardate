import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError, NETWORK_ERROR_STATUS, api, setErrorHandler } from './client';

const fetchMock = vi.fn<typeof fetch>();
const errorHandler = vi.fn<(error: ApiError) => void>();

function jsonResponse(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' }
	});
}

beforeEach(() => {
	vi.stubGlobal('fetch', fetchMock);
	setErrorHandler(errorHandler);
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.resetAllMocks();
});

describe('api client', () => {
	it('returns the parsed body of a successful response', async () => {
		fetchMock.mockResolvedValue(jsonResponse([{ jobId: 1 }]));

		await expect(api.get('/status')).resolves.toEqual([{ jobId: 1 }]);
		expect(fetchMock).toHaveBeenCalledWith(
			'/api/status',
			expect.objectContaining({ method: 'GET' })
		);
	});

	it('sends a JSON body and the version as a quoted If-Match', async () => {
		fetchMock.mockResolvedValue(jsonResponse({ id: 1 }));

		await api.put('/jobs/1', { body: { name: 'Acme' }, ifMatch: 3 });

		const init = fetchMock.mock.calls[0][1];
		const headers = new Headers(init?.headers);
		expect(init?.body).toBe('{"name":"Acme"}');
		expect(headers.get('Content-Type')).toBe('application/json');
		expect(headers.get('If-Match')).toBe('"3"');
	});

	it('returns undefined for 204 No Content', async () => {
		fetchMock.mockResolvedValue(new Response(null, { status: 204 }));

		await expect(api.delete('/jobs/1', { ifMatch: 2 })).resolves.toBeUndefined();
	});

	it("throws the backend's message and reports it once", async () => {
		fetchMock.mockResolvedValue(jsonResponse({ message: 'already clocked in on 2026-10-25' }, 409));

		const error = await api.post('/jobs/1/clock-in').catch((e: unknown) => e);

		expect(error).toBeInstanceOf(ApiError);
		expect(error).toMatchObject({ status: 409, message: 'already clocked in on 2026-10-25' });
		expect(errorHandler).toHaveBeenCalledExactlyOnceWith(error);
	});

	it('falls back to a generic message when the error body is not JSON', async () => {
		fetchMock.mockResolvedValue(new Response('<h1>Bad Gateway</h1>', { status: 502 }));

		const error = await api.get('/status').catch((e: unknown) => e);

		expect(error).toMatchObject({
			status: 502,
			message: 'Something went wrong. Please try again.'
		});
		expect(errorHandler).toHaveBeenCalledOnce();
	});

	it('reports a network failure with status 0', async () => {
		fetchMock.mockRejectedValue(new TypeError('Failed to fetch'));

		const error = await api.get('/status').catch((e: unknown) => e);

		expect(error).toMatchObject({
			status: NETWORK_ERROR_STATUS,
			message: 'Cannot reach the server. Check your connection.'
		});
		expect(errorHandler).toHaveBeenCalledOnce();
	});

	it('rethrows an aborted request without reporting it', async () => {
		const abort = new DOMException('The operation was aborted.', 'AbortError');
		fetchMock.mockRejectedValue(abort);

		await expect(api.get('/status', { signal: AbortSignal.abort() })).rejects.toBe(abort);
		expect(errorHandler).not.toHaveBeenCalled();
	});
});
