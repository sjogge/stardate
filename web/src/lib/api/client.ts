const API_BASE = '/api';
const GENERIC_MESSAGE = 'Something went wrong. Please try again.';
const NETWORK_MESSAGE = 'Cannot reach the server. Check your connection.';
/** Status of an `ApiError` when no response arrived at all */
export const NETWORK_ERROR_STATUS = 0;

export class ApiError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
		this.name = 'ApiError';
	}
}

type ErrorHandler = (error: ApiError) => void;

let errorHandler: ErrorHandler | undefined;

/** One global handler (the root layout's toaster), so feature code never reports errors itself. */
export function setErrorHandler(handler: ErrorHandler): void {
	errorHandler = handler;
}

export interface RequestOptions {
	body?: unknown;
	/** The resource version for `PUT` and `DELETE` on versioned resources */
	ifMatch?: number;
	signal?: AbortSignal;
}

async function request<T>(method: string, path: string, options: RequestOptions = {}): Promise<T> {
	const headers = new Headers({ Accept: 'application/json' });
	if (options.body !== undefined) headers.set('Content-Type', 'application/json');
	if (options.ifMatch !== undefined) headers.set('If-Match', `"${options.ifMatch}"`);

	let response: Response;
	try {
		response = await fetch(API_BASE + path, {
			method,
			headers,
			body: options.body === undefined ? undefined : JSON.stringify(options.body),
			signal: options.signal
		});
	} catch (error) {
		// Aborting is deliberate (e.g. leaving a page), so it is not reported
		if (isAbort(error)) throw error;
		throw report(new ApiError(NETWORK_ERROR_STATUS, NETWORK_MESSAGE));
	}

	if (!response.ok) {
		throw report(new ApiError(response.status, await readMessage(response)));
	}
	if (response.status === 204) return undefined as T;

	try {
		return (await response.json()) as T;
	} catch (error) {
		if (isAbort(error)) throw error;
		throw report(new ApiError(response.status, GENERIC_MESSAGE));
	}
}

/**
 * The backend answers every error with `{"message": "..."}` (backend/src/error.rs).
 * Anything else (e.g. an HTML error page from a proxy) gets the generic message.
 */
async function readMessage(response: Response): Promise<string> {
	try {
		const body: unknown = await response.json();
		if (
			typeof body === 'object' &&
			body !== null &&
			'message' in body &&
			typeof body.message === 'string' &&
			body.message.length > 0
		) {
			return body.message;
		}
	} catch {
		// Not JSON: fall through to the generic message
	}
	return GENERIC_MESSAGE;
}

function report(error: ApiError): ApiError {
	errorHandler?.(error);
	return error;
}

function isAbort(error: unknown): boolean {
	return error instanceof DOMException && error.name === 'AbortError';
}

export const api = {
	get: <T>(path: string, options?: Pick<RequestOptions, 'signal'>) =>
		request<T>('GET', path, options),
	post: <T>(path: string, options?: RequestOptions) => request<T>('POST', path, options),
	put: <T>(path: string, options?: RequestOptions) => request<T>('PUT', path, options),
	delete: (path: string, options?: RequestOptions) => request<void>('DELETE', path, options)
};
