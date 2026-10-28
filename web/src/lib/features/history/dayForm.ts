import type { Day, DayKind, PauseKind } from '$lib/api/types';
import {
	formatDuration,
	localIsoDate,
	localTimeOfDay,
	parseDuration,
	toInstant
} from '$lib/format';
import { PAUSE_NAMES } from '$lib/features/workday/actions';

/**
 * One pause as the form holds it: either a start and end time (`HH:MM`), or a duration such as
 * `0:45` when only the length is known. An empty end on an open day is a pause still running.
 */
export interface PauseDraft {
	kind: PauseKind;
	start: string;
	end: string;
	duration: string;
}

export interface DayDraft {
	workDate: string;
	kind: DayKind;
	/** Local times `HH:MM` on the work date; an end before the start is on the next day */
	start: string;
	/** Empty while the day is still open */
	end: string;
	pauses: PauseDraft[];
	note: string;
	/** Nothing expected that day (e.g. work on a public holiday): all worked time counts as plus */
	bonus: boolean;
	/** Whether the day was registered with nothing expected; undefined for a new day */
	registeredAsBonus: boolean | undefined;
	/** The schedule the day is worked on; undefined until chosen */
	scheduleId: number | undefined;
	/**
	 * The day's schedule as registered: undefined for a new day, null if that schedule was deleted.
	 * Choosing another one re-takes the expected times.
	 */
	registeredScheduleId: number | null | undefined;
	/** Re-take the expected times from the same schedule, e.g. after editing it */
	reapply: boolean;
}

/** Expected values that make a day a bonus day; the paid flags stay as the schedule has them */
const NOTHING_EXPECTED = { workMinutes: 0, lunchMinutes: 0, breakMinutes: 0 } as const;

export interface DayRequest {
	kind: DayKind;
	startedAt?: string;
	endedAt?: string;
	pauses: PauseRequest[];
	note: string | null;
	/** Required for a new day; on an existing day it re-takes the expected values from it */
	scheduleId?: number;
	expected?: typeof NOTHING_EXPECTED;
}

export interface PauseRequest {
	kind: PauseKind;
	startedAt?: string;
	endedAt?: string;
	minutes?: number;
}

export function emptyPause(kind: PauseKind = 'lunch'): PauseDraft {
	return { kind, start: '', end: '', duration: '' };
}

/** A form filled from `day`, or an empty work day on `workDate` on `scheduleId` (the suggestion) */
export function draftFromDay(
	day: Day | undefined,
	workDate: string,
	scheduleId?: number
): DayDraft {
	if (!day) {
		return {
			workDate,
			kind: 'work',
			start: '',
			end: '',
			pauses: [],
			note: '',
			bonus: false,
			registeredAsBonus: undefined,
			scheduleId,
			registeredScheduleId: undefined,
			reapply: false
		};
	}
	// Also true for a weekday the schedule has no hours on (e.g. a Saturday): that is bonus time too
	const bonus =
		day.kind === 'work' &&
		day.expected.workMinutes === 0 &&
		day.expected.lunchMinutes === 0 &&
		day.expected.breakMinutes === 0;
	return {
		workDate: day.workDate,
		kind: day.kind,
		start: localTime(day.startedAt),
		end: localTime(day.endedAt),
		pauses: day.pauses.map((pause) => ({
			kind: pause.kind,
			start: localTime(pause.startedAt),
			end: localTime(pause.endedAt),
			duration: pause.minutes === null ? '' : formatDuration(pause.minutes)
		})),
		note: day.note ?? '',
		bonus,
		registeredAsBonus: bonus,
		scheduleId: day.scheduleId ?? undefined,
		registeredScheduleId: day.scheduleId,
		reapply: false
	};
}

/**
 * Whether the request names the schedule, so the backend takes the expected values from it: always
 * for a new day, and on an edit when another schedule is chosen, re-applying is asked for, or a
 * bonus day stops being one (it gets the schedule's times back).
 */
function takesSchedule(draft: DayDraft, bonus: boolean): boolean {
	if (draft.registeredScheduleId === undefined) return true;
	return (
		draft.scheduleId !== draft.registeredScheduleId ||
		draft.reapply ||
		(draft.registeredAsBonus === true && !bonus)
	);
}

/** The request body, or the first problem a person can fix in the form; the backend checks the rest */
export function toDayRequest(draft: DayDraft): { request: DayRequest } | { error: string } {
	if (draft.workDate === '') return { error: 'Choose the date.' };
	const note = draft.note.trim() === '' ? null : draft.note.trim();
	const bonus = draft.kind === 'work' && draft.bonus;
	let expectation: Pick<DayRequest, 'scheduleId' | 'expected'> = {};
	if (takesSchedule(draft, bonus)) {
		if (draft.scheduleId === undefined)
			return { error: 'Choose the schedule this day is worked on.' };
		expectation = { scheduleId: draft.scheduleId };
	}
	// After any new snapshot, so a bonus day keeps nothing expected
	if (bonus) expectation = { ...expectation, expected: NOTHING_EXPECTED };
	if (draft.kind === 'flex') {
		return { request: { kind: 'flex', pauses: [], note, ...expectation } };
	}

	const startedAt = instantOnDay(draft.workDate, draft.start);
	if (!startedAt) return { error: 'Enter the start time.' };
	const endedAt = draft.end === '' ? undefined : instantAfter(startedAt, draft.workDate, draft.end);

	const pauses: PauseRequest[] = [];
	for (const [index, pause] of draft.pauses.entries()) {
		const name = `Pause ${index + 1} (${PAUSE_NAMES[pause.kind]})`;
		if (pause.duration.trim() !== '') {
			const minutes = parseDuration(pause.duration);
			if (minutes === undefined) return { error: `${name}: use hours and minutes, e.g. 0:45.` };
			pauses.push({ kind: pause.kind, minutes });
			continue;
		}
		const pauseStart =
			pause.start === '' ? undefined : instantAfter(startedAt, draft.workDate, pause.start);
		if (!pauseStart) return { error: `${name}: enter a start time or a duration.` };
		const pauseEnd =
			pause.end === '' ? undefined : instantAfter(startedAt, draft.workDate, pause.end);
		pauses.push({ kind: pause.kind, startedAt: pauseStart, endedAt: pauseEnd });
	}

	return { request: { kind: 'work', startedAt, endedAt, pauses, note, ...expectation } };
}

function instantOnDay(workDate: string, time: string): string | undefined {
	return time === '' ? undefined : toInstant({ date: workDate, time });
}

/**
 * The time on the work date, or on the next day if that is before `dayStart`: a day that
 * starts at 22:00 and ends at 06:00 ends the next morning.
 */
function instantAfter(dayStart: string, workDate: string, time: string): string | undefined {
	const instant = instantOnDay(workDate, time);
	if (!instant || instant >= dayStart) return instant;
	// The next calendar date, not 24 hours later: a night can cross a daylight saving change
	const [year, month, day] = workDate.split('-').map(Number);
	return instantOnDay(localIsoDate(new Date(year, month - 1, day + 1)), time);
}

function localTime(instant: string | null): string {
	return instant ? localTimeOfDay(new Date(instant)) : '';
}
