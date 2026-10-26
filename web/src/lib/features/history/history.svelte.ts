import { api } from '$lib/api/client';
import type { Day, Job, RangeTotals, Summary } from '$lib/api/types';
import { todayIsoDate } from '$lib/format';
import { isSameMonth, monthOf, monthRange, shiftMonth, type Month } from './month';

/** State of the history screen: one job's days and totals for one month. */
export class History {
	jobs = $state<Job[]>([]);
	jobId = $state<number>();
	month = $state<Month>(monthOf(todayIsoDate()));
	/** Oldest first, as the API returns them */
	days = $state<Day[]>([]);
	totals = $state<RangeTotals>();
	/** A load is in flight */
	loading = $state(true);
	failed = $state(false);

	#controller: AbortController | undefined;

	get newestFirst(): Day[] {
		return this.days.toReversed();
	}

	/** Later months have no registered days yet, so the screen does not page past this one */
	get isCurrentMonth(): boolean {
		return isSameMonth(this.month, monthOf(todayIsoDate()));
	}

	/** Loads the jobs, picks the first active one (or any, if all are archived), then its month */
	async init(): Promise<void> {
		try {
			this.jobs = await api.get<Job[]>('/jobs');
		} catch {
			// Already toasted by the client's error handler
			this.failed = true;
			this.loading = false;
			return;
		}
		this.jobId = (this.jobs.find((job) => job.archivedAt === null) ?? this.jobs[0])?.id;
		await this.load();
	}

	selectJob(jobId: number): Promise<void> {
		this.jobId = jobId;
		return this.load();
	}

	showMonth(by: number): Promise<void> {
		this.month = shiftMonth(this.month, by);
		return this.load();
	}

	async load(): Promise<void> {
		if (this.jobId === undefined) {
			this.loading = false;
			return;
		}
		// Paging quickly through months: only the last request's answer is shown
		this.#controller?.abort();
		const controller = new AbortController();
		this.#controller = controller;

		const { from, to } = monthRange(this.month);
		const job = `/jobs/${this.jobId}`;
		const options = { signal: controller.signal };
		this.loading = true;
		try {
			const [days, summary] = await Promise.all([
				api.get<Day[]>(`${job}/days?from=${from}&to=${to}`, options),
				api.get<Summary>(`${job}/summary?from=${from}&to=${to}`, options)
			]);
			this.days = days;
			this.totals = summary.totals;
			this.failed = false;
		} catch {
			// Already toasted; an aborted load is followed by a newer one
			if (!controller.signal.aborted) this.failed = true;
		} finally {
			if (!controller.signal.aborted) this.loading = false;
		}
	}
}
