import type { Schedule } from '$lib/api/types';
import { formatDuration, formatWeekday, parseDuration } from '$lib/format';

export const WEEKDAYS = [1, 2, 3, 4, 5, 6, 7] as const;

/** What the form holds per weekday: clock times from time inputs, pauses as typed lengths */
export interface ShiftDraft {
	/** `HH:MM`, or empty for a day without work */
	start: string;
	end: string;
	/** e.g. `1:00` */
	lunch: string;
	breaks: string;
}

export interface ScheduleDraft {
	name: string;
	/** Undefined until chosen: paid or unpaid must be a deliberate choice (no default) */
	lunchPaid: boolean | undefined;
	breaksPaid: boolean | undefined;
	/** The schedule of the following week; null repeats this one */
	nextScheduleId: number | null;
	/** Index 0 is Monday */
	days: ShiftDraft[];
}

export interface ScheduleDayRequest {
	weekday: number;
	start: string;
	end: string;
	lunchMinutes: number;
	breakMinutes: number;
}

export interface ScheduleRequest {
	name: string;
	lunchPaid: boolean;
	breaksPaid: boolean;
	nextScheduleId: number | null;
	days: ScheduleDayRequest[];
}

const EMPTY_SHIFT: ShiftDraft = { start: '', end: '', lunch: '', breaks: '' };

/** A form filled from `schedule`, or an empty one */
export function draftFrom(schedule: Schedule | undefined): ScheduleDraft {
	return {
		name: schedule?.name ?? '',
		lunchPaid: schedule?.lunchPaid,
		breaksPaid: schedule?.breaksPaid,
		nextScheduleId: schedule?.nextScheduleId ?? null,
		days: WEEKDAYS.map((weekday) => {
			const day = schedule?.days.find((scheduled) => scheduled.weekday === weekday);
			if (!day) return { ...EMPTY_SHIFT };
			return {
				start: day.start,
				end: day.end,
				lunch: formatDuration(day.lunchMinutes),
				breaks: formatDuration(day.breakMinutes)
			};
		})
	};
}

/** A new schedule with the same shifts, e.g. as the starting point for the next rotation week */
export function duplicateDraft(schedule: Schedule): ScheduleDraft {
	return { ...draftFrom(schedule), name: `${schedule.name} (copy)`, nextScheduleId: null };
}

/** Copies Monday's shift to Tuesday to Friday, the usual working week */
export function copyMondayToWeekdays(draft: ScheduleDraft): void {
	const [monday] = draft.days;
	for (let index = 1; index <= 4; index++) draft.days[index] = { ...monday };
}

/**
 * The request body, or the first problem a person can fix in the form; the backend checks the
 * rest (e.g. that lunch and breaks fit in the shift). A weekday with all fields empty is left
 * out: no work is expected that day.
 */
export function toRequest(draft: ScheduleDraft): { request: ScheduleRequest } | { error: string } {
	const name = draft.name.trim();
	if (name === '') return { error: 'Give the schedule a name, e.g. Daytime.' };
	if (draft.lunchPaid === undefined) return { error: 'Choose whether lunch is paid.' };
	if (draft.breaksPaid === undefined) return { error: 'Choose whether breaks are paid.' };

	const days: ScheduleDayRequest[] = [];
	for (const [index, shift] of draft.days.entries()) {
		if (Object.values(shift).every((text) => text.trim() === '')) continue;

		const weekday = index + 1;
		const day = formatWeekday(weekday);
		if (shift.start === '' || shift.end === '') {
			return { error: `${day}: enter both start and end, or clear the day.` };
		}
		if (shift.start === shift.end) {
			return { error: `${day}: start and end must differ.` };
		}
		const lunchMinutes = parseDuration(shift.lunch);
		const breakMinutes = parseDuration(shift.breaks);
		if (lunchMinutes === undefined || breakMinutes === undefined) {
			return { error: `${day}: enter lunch and breaks in hours and minutes, e.g. 0:30.` };
		}
		days.push({ weekday, start: shift.start, end: shift.end, lunchMinutes, breakMinutes });
	}

	const { lunchPaid, breaksPaid, nextScheduleId } = draft;
	return { request: { name, lunchPaid, breaksPaid, nextScheduleId, days } };
}
