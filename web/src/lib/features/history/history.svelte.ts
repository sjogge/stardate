import { api } from '$lib/api/client';
import type { Day, FlexAdjustment, FlexBalance, Job, RangeTotals, Summary } from '$lib/api/types';
import { todayIsoDate } from '$lib/format';
import { showSuccess } from '$lib/toast';
import type { AdjustmentRequest } from './adjustmentForm';
import type { DayRequest } from './dayForm';
import { isSameMonth, monthOf, monthRange, shiftMonth, type Month } from './month';

/** State of the history screen: one job's days and totals for one month, and its flex balance. */
export class History {
	jobs = $state<Job[]>([]);
	jobId = $state<number>();
	month = $state<Month>(monthOf(todayIsoDate()));
	/** Oldest first, as the API returns them */
	days = $state<Day[]>([]);
	totals = $state<RangeTotals>();
	/** All-time, not just the month: closed days plus every adjustment */
	flex = $state<FlexBalance>();
	/** Oldest first, as the API returns them */
	adjustments = $state<FlexAdjustment[]>([]);
	/** A load is in flight */
	loading = $state(true);
	failed = $state(false);
	/** A change is being sent; its buttons are disabled */
	pending = $state(false);

	#controller: AbortController | undefined;

	get newestFirst(): Day[] {
		return this.days.toReversed();
	}

	get newestAdjustmentsFirst(): FlexAdjustment[] {
		return this.adjustments.toReversed();
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

	/** Where "Add day" starts: today in the current month, else the shown month's last day */
	get newDayDate(): string {
		return this.isCurrentMonth ? todayIsoDate() : monthRange(this.month).to;
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
			// Loaded together: a change to a day or an adjustment moves the month and the balance
			const [days, summary, flex, adjustments] = await Promise.all([
				api.get<Day[]>(`${job}/days?from=${from}&to=${to}`, options),
				api.get<Summary>(`${job}/summary?from=${from}&to=${to}`, options),
				api.get<FlexBalance>(`${job}/flex`, options),
				api.get<FlexAdjustment[]>(`${job}/flex-adjustments`, options)
			]);
			this.days = days;
			this.totals = summary.totals;
			this.flex = flex;
			this.adjustments = adjustments;
			this.failed = false;
		} catch {
			// Already toasted; an aborted load is followed by a newer one
			if (!controller.signal.aborted) this.failed = true;
		} finally {
			if (!controller.signal.aborted) this.loading = false;
		}
	}

	/** Creates the day on `workDate`, or replaces `existing` if given */
	save(workDate: string, request: DayRequest, existing?: Day): Promise<boolean> {
		const path = `/jobs/${this.jobId}/days/${workDate}`;
		return this.#change(
			() =>
				existing
					? api.put(path, { body: request, ifMatch: existing.version })
					: api.put(path, { body: request, createOnly: true }),
			existing ? 'Day saved' : 'Day added'
		);
	}

	remove(day: Day): Promise<boolean> {
		return this.#change(
			() => api.delete(`/jobs/${this.jobId}/days/${day.workDate}`, { ifMatch: day.version }),
			'Day deleted'
		);
	}

	/** Adds an adjustment, or replaces `existing` if given */
	saveAdjustment(request: AdjustmentRequest, existing?: FlexAdjustment): Promise<boolean> {
		const path = `/jobs/${this.jobId}/flex-adjustments`;
		return this.#change(
			() =>
				existing
					? api.put(`${path}/${existing.id}`, { body: request, ifMatch: existing.version })
					: api.post(path, { body: request }),
			existing ? 'Adjustment saved' : 'Flex balance adjusted'
		);
	}

	removeAdjustment(adjustment: FlexAdjustment): Promise<boolean> {
		return this.#change(
			() =>
				api.delete(`/jobs/${this.jobId}/flex-adjustments/${adjustment.id}`, {
					ifMatch: adjustment.version
				}),
			'Adjustment deleted'
		);
	}

	/** Sends a change and reloads the month; resolves to whether the change went through */
	async #change(send: () => Promise<unknown>, successMessage: string): Promise<boolean> {
		if (this.pending || this.jobId === undefined) return false;
		this.pending = true;
		try {
			await send();
			showSuccess(successMessage);
			return true;
		} catch {
			// Already toasted; e.g. a day already registered on that date (412) or invalid times (400)
			return false;
		} finally {
			await this.load();
			this.pending = false;
		}
	}
}
