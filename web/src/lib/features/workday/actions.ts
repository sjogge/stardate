import { api } from '$lib/api/client';
import type { Day, JobStatus, PauseKind } from '$lib/api/types';

/** A live action as the screen offers it: the backend's action plus the pause kind it needs */
export type WorkdayAction =
	| { type: 'clockIn' }
	| { type: 'clockOut' }
	| { type: 'startPause'; pause: PauseKind }
	| { type: 'endPause'; pause: PauseKind };

export interface ActionChoice {
	/** The large button: the most likely next step */
	primary: WorkdayAction | undefined;
	secondary: WorkdayAction[];
}

/** How each pause kind is called in the UI ('other' is an absence, e.g. a dentist visit) */
export const PAUSE_NAMES: Record<PauseKind, string> = {
	lunch: 'lunch',
	break: 'break',
	other: 'absence'
};

/**
 * Which buttons to show and which one is primary. The backend decides what is allowed
 * (`availableActions`); picking the primary one is presentation.
 */
export function chooseActions(status: JobStatus): ActionChoice {
	const allowed = new Set(status.availableActions);

	if (allowed.has('clockIn')) {
		return { primary: { type: 'clockIn' }, secondary: [] };
	}

	const clockOut: WorkdayAction[] = allowed.has('clockOut') ? [{ type: 'clockOut' }] : [];

	if (allowed.has('endPause') && status.runningPause) {
		return {
			primary: { type: 'endPause', pause: status.runningPause.kind },
			secondary: clockOut
		};
	}

	if (allowed.has('startPause')) {
		const pauses: WorkdayAction[] = (['lunch', 'break', 'other'] as const).map((pause) => ({
			type: 'startPause',
			pause
		}));
		if (lunchIsDue(status.today)) {
			const [lunch, ...otherPauses] = pauses;
			return { primary: lunch, secondary: [...otherPauses, ...clockOut] };
		}
		return { primary: clockOut[0], secondary: pauses };
	}

	return { primary: undefined, secondary: [] };
}

/** Lunch is expected today and none has been logged yet */
function lunchIsDue(today: Day | null): boolean {
	if (!today || today.expected.lunchMinutes === 0) return false;
	return !today.pauses.some((pause) => pause.kind === 'lunch');
}

export function actionLabel(action: WorkdayAction): string {
	switch (action.type) {
		case 'clockIn':
			return 'Clock in';
		case 'clockOut':
			return 'Clock out';
		case 'startPause':
			return `Start ${PAUSE_NAMES[action.pause]}`;
		case 'endPause':
			return `End ${PAUSE_NAMES[action.pause]}`;
	}
}

/** The short success message after the action went through */
export function actionDoneMessage(action: WorkdayAction): string {
	switch (action.type) {
		case 'clockIn':
			return 'Clocked in';
		case 'clockOut':
			return 'Clocked out';
		case 'startPause':
			return `${capitalize(PAUSE_NAMES[action.pause])} started`;
		case 'endPause':
			return `${capitalize(PAUSE_NAMES[action.pause])} ended`;
	}
}

function capitalize(text: string): string {
	return text.charAt(0).toUpperCase() + text.slice(1);
}

/** Sends the action; `workDate` is the client's local date, which clock-in needs. */
export function runAction(jobId: number, action: WorkdayAction, workDate: string): Promise<Day> {
	const job = `/jobs/${jobId}`;
	switch (action.type) {
		case 'clockIn':
			return api.post<Day>(`${job}/clock-in`, { body: { workDate } });
		case 'clockOut':
			return api.post<Day>(`${job}/clock-out`, { body: {} });
		case 'startPause':
			return api.post<Day>(`${job}/pauses/start`, { body: { kind: action.pause } });
		case 'endPause':
			return api.post<Day>(`${job}/pauses/end`, { body: {} });
	}
}
