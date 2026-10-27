import type { Day } from '$lib/api/types';
import { formatTime } from '$lib/format';

/** When the day was worked, e.g. `08:00–17:00`; an open day has no end yet */
export function dayTimes(day: Day): string {
	if (day.kind === 'flex') return 'Flex day';
	if (!day.startedAt) return '';
	const start = formatTime(day.startedAt);
	return day.endedAt ? `${start}–${formatTime(day.endedAt)}` : `${start}–(open)`;
}
