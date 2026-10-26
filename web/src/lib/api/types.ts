// Response shapes of the backend (backend/src/days.rs, status.rs, live.rs).
// Instants are RFC 3339 strings in UTC; dates are `YYYY-MM-DD`; durations are whole minutes.

export type DayKind = 'work' | 'flex';
export type PauseKind = 'lunch' | 'break' | 'other';
export type LiveAction = 'clockIn' | 'clockOut' | 'startPause' | 'endPause';

export interface Expected {
	workMinutes: number;
	lunchMinutes: number;
	breakMinutes: number;
	lunchPaid: boolean;
	breaksPaid: boolean;
}

/** Either timed (`startedAt`, plus `endedAt` once finished) or a plain duration (`minutes`) */
export interface Pause {
	id: number;
	kind: PauseKind;
	startedAt: string | null;
	endedAt: string | null;
	minutes: number | null;
}

export interface DayTotals {
	presenceMinutes: number;
	/** Actual minutes, defaulting to the expected ones when none were logged */
	lunchMinutes: number;
	breakMinutes: number;
	otherMinutes: number;
	allowanceMinutes: number;
	workedMinutes: number;
	expectedWorkMinutes: number;
	balanceMinutes: number;
	/** Only while the day is open */
	leaveAt: string | null;
}

export interface Day {
	id: number;
	workDate: string;
	kind: DayKind;
	startedAt: string | null;
	endedAt: string | null;
	note: string | null;
	scheduleId: number | null;
	expected: Expected;
	pauses: Pause[];
	totals: DayTotals;
	version: number;
	createdAt: string;
	updatedAt: string;
}

export interface RunningPause {
	kind: PauseKind;
	startedAt: string;
}

export interface JobStatus {
	jobId: number;
	jobName: string;
	clockedIn: boolean;
	runningPause: RunningPause | null;
	availableActions: LiveAction[];
	/** The open day, or else the day registered on the requested date */
	today: Day | null;
	/** Closed days and all adjustments; an open day counts once clocked out */
	flexBalanceMinutes: number;
}
