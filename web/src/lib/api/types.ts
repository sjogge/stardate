// Response shapes of the backend (backend/src/days.rs, status.rs, live.rs).
// Instants are RFC 3339 strings in UTC; dates are `YYYY-MM-DD`; durations are whole minutes.

export interface Job {
	id: number;
	name: string;
	/** Set once archived: the job is kept for its history but takes no new days */
	archivedAt: string | null;
	version: number;
	createdAt: string;
	updatedAt: string;
}

export interface ScheduleDay {
	/** ISO 8601 weekday: 1 = Monday, 7 = Sunday */
	weekday: number;
	workMinutes: number;
	lunchMinutes: number;
	breakMinutes: number;
}

/** One version of a job's schedule: it applies from `validFrom` until the next version */
export interface Schedule {
	id: number;
	validFrom: string;
	lunchPaid: boolean;
	breaksPaid: boolean;
	/** Sorted by weekday; a missing weekday means no expected work that day */
	days: ScheduleDay[];
	version: number;
	createdAt: string;
	updatedAt: string;
}

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

/** Sums of the closed days in a range, plus the flex adjustments dated in it */
export interface RangeTotals {
	workDays: number;
	flexDays: number;
	workedMinutes: number;
	expectedWorkMinutes: number;
	daysBalanceMinutes: number;
	adjustmentsMinutes: number;
	balanceMinutes: number;
}

/** `GET /jobs/{id}/summary`; only the totals are used, the days come from `/days` */
export interface Summary {
	from: string;
	to: string;
	totals: RangeTotals;
}

/** `GET /jobs/{id}/flex`: closed days plus every adjustment */
export interface FlexBalance {
	balanceMinutes: number;
	daysMinutes: number;
	adjustmentsMinutes: number;
}

/** A manual change to the flex balance, e.g. hours carried over from an old system */
export interface FlexAdjustment {
	id: number;
	adjustedOn: string;
	/** Positive adds to the balance, negative takes from it */
	minutes: number;
	note: string | null;
	version: number;
	createdAt: string;
	updatedAt: string;
}
