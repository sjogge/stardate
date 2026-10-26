// Display formatting only: the values themselves come from the API.
//
// The UI text is English, so weekday and month names are English too. Clock times and the
// native date inputs follow the device's locale (24-hour or am/pm, date order).

const MINUTES_PER_HOUR = 60;

const MAX_MINUTES_PER_DAY = 24 * MINUTES_PER_HOUR;
/** `8`, `8:00`, `7:30` or `0:45` */
const DURATION_PATTERN = /^(\d{1,2})(?::([0-5]\d))?$/;

/**
 * The locale for dates written with words: the device's own English variant (en-US: `Oct 26`),
 * otherwise British English (`26 Oct`), whose day-month order suits most other locales.
 */
export function textLocale(): string {
	const deviceLocale = new Intl.DateTimeFormat().resolvedOptions().locale;
	return deviceLocale.startsWith('en') ? deviceLocale : 'en-GB';
}

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

/** The local time of day as `HH:MM`, the format of `<input type="time">` */
export function localTimeOfDay(date: Date): string {
	const hours = String(date.getHours()).padStart(2, '0');
	const minutes = String(date.getMinutes()).padStart(2, '0');
	return `${hours}:${minutes}`;
}

export interface LocalDateTime {
	/** `YYYY-MM-DD` */
	date: string;
	/** `HH:MM` */
	time: string;
}

export function currentLocalDateTime(): LocalDateTime {
	const now = new Date();
	return { date: localIsoDate(now), time: localTimeOfDay(now) };
}

/**
 * A local date and time as an RFC 3339 instant (in UTC), or undefined if either is incomplete.
 * A date-time string without an offset is read in the device's time zone.
 */
export function toInstant({ date, time }: LocalDateTime): string | undefined {
	const instant = new Date(`${date}T${time}`);
	return Number.isNaN(instant.getTime()) ? undefined : instant.toISOString();
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

/** An RFC 3339 instant as a clock time in the device's locale, e.g. `08:02` or `8:02 AM` */
export function formatTime(instant: string, locale?: string, timeZone?: string): string {
	return new Intl.DateTimeFormat(locale, { timeStyle: 'short', timeZone }).format(
		new Date(instant)
	);
}

/** A `YYYY-MM-DD` date as a short English date with weekday, e.g. `Sun 25 Oct` */
export function formatShortDate(isoDate: string, locale = textLocale()): string {
	// Without a time, `Date` would read the date as UTC midnight, which can be the day before
	return new Intl.DateTimeFormat(locale, {
		weekday: 'short',
		day: 'numeric',
		month: 'short'
	}).format(new Date(`${isoDate}T00:00`));
}

/** A `YYYY-MM-DD` date as an English date with year, e.g. `26 Oct 2026` */
export function formatDate(isoDate: string, locale = textLocale()): string {
	return new Intl.DateTimeFormat(locale, { dateStyle: 'medium' }).format(
		new Date(`${isoDate}T00:00`)
	);
}

/** A month as English text, e.g. `October 2026`; `month` is 1 for January */
export function formatMonth(year: number, month: number, locale = textLocale()): string {
	return new Intl.DateTimeFormat(locale, { month: 'long', year: 'numeric' }).format(
		new Date(year, month - 1, 1)
	);
}

/** An ISO weekday (1 = Monday, 7 = Sunday) as a short English name, e.g. `Mon` */
export function formatWeekday(isoWeekday: number, locale = textLocale()): string {
	// January 2024 started on a Monday, so day N of it has ISO weekday N
	return new Intl.DateTimeFormat(locale, { weekday: 'short' }).format(
		new Date(2024, 0, isoWeekday)
	);
}

/**
 * Reads a typed duration (`8`, `8:00`, `0:45`) as minutes; an empty field is 0.
 * Undefined if it is not a duration of at most 24 hours.
 */
export function parseDuration(text: string): number | undefined {
	const trimmed = text.trim();
	if (trimmed === '') return 0;
	const match = DURATION_PATTERN.exec(trimmed);
	if (!match) return undefined;
	const minutes = Number(match[1]) * MINUTES_PER_HOUR + Number(match[2] ?? 0);
	return minutes <= MAX_MINUTES_PER_DAY ? minutes : undefined;
}
