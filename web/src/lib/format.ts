// Display formatting only: the values themselves come from the API.

const MINUTES_PER_HOUR = 60;

/** The local calendar date as `YYYY-MM-DD`, which the API expects for "today" */
export function localIsoDate(date: Date): string {
	const year = date.getFullYear();
	const month = String(date.getMonth() + 1).padStart(2, '0');
	const day = String(date.getDate()).padStart(2, '0');
	return `${year}-${month}-${day}`;
}

/** Today's local date: what the API calls "today" for this device */
export function todayIsoDate(): string {
	return localIsoDate(new Date());
}

/** `7:05` for 425 minutes; `-0:30` for -30 */
export function formatDuration(minutes: number): string {
	const sign = minutes < 0 ? '-' : '';
	const absolute = Math.abs(minutes);
	const hours = Math.floor(absolute / MINUTES_PER_HOUR);
	const rest = String(absolute % MINUTES_PER_HOUR).padStart(2, '0');
	return `${sign}${hours}:${rest}`;
}

/** Like `formatDuration`, with `+` for a surplus, so a balance reads as one */
export function formatBalance(minutes: number): string {
	return minutes > 0 ? `+${formatDuration(minutes)}` : formatDuration(minutes);
}

/** An RFC 3339 instant as a local clock time, e.g. `08:02` */
export function formatTime(instant: string, locale?: string, timeZone?: string): string {
	return new Intl.DateTimeFormat(locale, { timeStyle: 'short', timeZone }).format(
		new Date(instant)
	);
}
