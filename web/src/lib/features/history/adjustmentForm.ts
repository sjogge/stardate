import type { FlexAdjustment } from '$lib/api/types';
import { formatDuration, parseDuration } from '$lib/format';

/** The backend's limit (flex.rs): a year of full days */
const MAX_ADJUSTMENT_MINUTES = 365 * 24 * 60;

export type Direction = 'add' | 'take';

export interface AdjustmentDraft {
	adjustedOn: string;
	direction: Direction;
	/** As typed, e.g. `2:00`; the sign comes from `direction` */
	amount: string;
	note: string;
}

export interface AdjustmentRequest {
	adjustedOn: string;
	/** Positive adds to the balance, negative takes from it */
	minutes: number;
	note: string | null;
}

export function draftFromAdjustment(
	adjustment: FlexAdjustment | undefined,
	adjustedOn: string
): AdjustmentDraft {
	if (!adjustment) return { adjustedOn, direction: 'add', amount: '', note: '' };
	return {
		adjustedOn: adjustment.adjustedOn,
		direction: adjustment.minutes < 0 ? 'take' : 'add',
		amount: formatDuration(Math.abs(adjustment.minutes)),
		note: adjustment.note ?? ''
	};
}

/** The request body, or the first problem a person can fix in the form */
export function toAdjustmentRequest(
	draft: AdjustmentDraft
): { request: AdjustmentRequest } | { error: string } {
	if (draft.adjustedOn === '') return { error: 'Choose the date.' };
	const minutes = parseDuration(draft.amount, MAX_ADJUSTMENT_MINUTES);
	if (minutes === undefined) return { error: 'Enter the time in hours and minutes, e.g. 2:30.' };
	if (minutes === 0) return { error: 'Enter how much time to add or take.' };
	const note = draft.note.trim() === '' ? null : draft.note.trim();
	const signed = draft.direction === 'take' ? -minutes : minutes;
	return { request: { adjustedOn: draft.adjustedOn, minutes: signed, note } };
}
