import { describe, expect, it } from 'vitest';
import type { FlexAdjustment } from '$lib/api/types';
import { draftFromAdjustment, toAdjustmentRequest } from './adjustmentForm';

const taken: FlexAdjustment = {
	id: 1,
	adjustedOn: '2026-10-01',
	minutes: -90,
	note: 'Paid out',
	version: 1,
	createdAt: '',
	updatedAt: ''
};

describe('adjustmentForm', () => {
	it('shows a negative adjustment as time taken and sends it back unchanged', () => {
		const draft = draftFromAdjustment(taken, 'ignored');

		expect(draft).toEqual({
			adjustedOn: '2026-10-01',
			direction: 'take',
			amount: '1:30',
			note: 'Paid out'
		});
		expect(toAdjustmentRequest(draft)).toEqual({
			request: { adjustedOn: '2026-10-01', minutes: -90, note: 'Paid out' }
		});
	});

	it('accepts amounts longer than a day', () => {
		const draft = draftFromAdjustment(undefined, '2026-10-27');
		draft.amount = '40:00';
		draft.note = '  ';

		expect(toAdjustmentRequest(draft)).toEqual({
			request: { adjustedOn: '2026-10-27', minutes: 2400, note: null }
		});
	});

	it('asks for an amount it can read', () => {
		const draft = draftFromAdjustment(undefined, '2026-10-27');
		expect(toAdjustmentRequest(draft)).toEqual({ error: 'Enter how much time to add or take.' });

		draft.amount = '2h';
		expect(toAdjustmentRequest(draft)).toEqual({
			error: 'Enter the time in hours and minutes, e.g. 2:30.'
		});
	});
});
