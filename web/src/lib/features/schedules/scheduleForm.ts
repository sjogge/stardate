import type { Schedule, ScheduleDay } from '$lib/api/types';
import { formatDuration, formatWeekday, parseDuration } from '$lib/format';

export const WEEKDAYS = [1, 2, 3, 4, 5, 6, 7] as const;

/** What the form holds per weekday: durations as typed, e.g. `8:00` */
export interface DayTimes {
	work: string;
	lunch: string;
	breaks: string;
}

export interface ScheduleDraft {
	validFrom: string;
	/** Undefined until chosen: paid or unpaid must be a deliberate choice (no default) */
	lunchPaid: boolean | undefined;
	breaksPaid: boolean | undefined;
	/** Index 0 is Monday */
	days: DayTimes[];
}

export interface ScheduleRequest {
	validFrom: string;
	lunchPaid: boolean;
	breaksPaid: boolean;
	days: ScheduleDay[];
}

const DAY_FIELDS = [
	['work', 'workMinutes', 'work'],
	['lunch', 'lunchMinutes', 'lunch'],
	['breaks', 'breakMinutes', 'break']
] as const;

/** A form filled from `schedule` (or empty), applying from `validFrom` */
export function draftFrom(schedule: Schedule | undefined, validFrom: string): ScheduleDraft {
	return {
		validFrom,
		lunchPaid: schedule?.lunchPaid,
		breaksPaid: schedule?.breaksPaid,
		days: WEEKDAYS.map((weekday) => {
			const day = schedule?.days.find((scheduled) => scheduled.weekday === weekday);
			return {
				work: day ? formatDuration(day.workMinutes) : '',
				lunch: day ? formatDuration(day.lunchMinutes) : '',
				breaks: day ? formatDuration(day.breakMinutes) : ''
			};
		})
	};
}

/** Copies Monday's times to Tuesday to Friday, the usual working week */
export function copyMondayToWeekdays(draft: ScheduleDraft): void {
	const [monday] = draft.days;
	for (let index = 1; index <= 4; index++) draft.days[index] = { ...monday };
}

/**
 * The request body, or the first problem a person can fix in the form. A weekday with all
 * fields empty is left out: no work is expected that day.
 */
export function toRequest(draft: ScheduleDraft): { request: ScheduleRequest } | { error: string } {
	if (draft.validFrom === '') return { error: 'Choose the date the schedule applies from.' };
	if (draft.lunchPaid === undefined) return { error: 'Choose whether lunch is paid.' };
	if (draft.breaksPaid === undefined) return { error: 'Choose whether breaks are paid.' };

	const days: ScheduleDay[] = [];
	for (const [index, times] of draft.days.entries()) {
		if (Object.values(times).every((text) => text.trim() === '')) continue;

		const weekday = index + 1;
		const day: ScheduleDay = { weekday, workMinutes: 0, lunchMinutes: 0, breakMinutes: 0 };
		for (const [field, key, name] of DAY_FIELDS) {
			const minutes = parseDuration(times[field]);
			if (minutes === undefined) {
				return {
					error: `${formatWeekday(weekday)} ${name}: use hours and minutes, e.g. 8:00 or 0:45.`
				};
			}
			day[key] = minutes;
		}
		days.push(day);
	}

	const { validFrom, lunchPaid, breaksPaid } = draft;
	return { request: { validFrom, lunchPaid, breaksPaid, days } };
}
