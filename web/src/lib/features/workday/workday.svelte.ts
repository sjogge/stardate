import { api } from '$lib/api/client';
import type { JobStatus } from '$lib/api/types';
import { currentLocalDateTime, toInstant, todayIsoDate, type LocalDateTime } from '$lib/format';
import { showSuccess } from '$lib/toast';
import { actionDoneMessage, runAction, type ActionTime, type WorkdayAction } from './actions';

/** State of the workday screen: every active job's status, and the job being shown. */
export class Workday {
	statuses = $state<JobStatus[]>([]);
	selectedJobId = $state<number>();
	/** True until the first load has finished */
	loading = $state(true);
	failed = $state(false);
	/** An action is being sent; its buttons are disabled */
	pending = $state(false);

	/** Set to back-date the next action (e.g. a forgotten clock-in); otherwise actions happen now */
	customTime = $state<LocalDateTime>();

	#loadController: AbortController | undefined;

	/** The chosen job; until one is chosen, the job you are clocked in on, else the first */
	get selected(): JobStatus | undefined {
		return (
			this.statuses.find((status) => status.jobId === this.selectedJobId) ??
			this.statuses.find((status) => status.clockedIn) ??
			this.statuses[0]
		);
	}

	/** False while a custom date or time is incomplete, so no action can be sent */
	get timeIsValid(): boolean {
		return this.customTime === undefined || toInstant(this.customTime) !== undefined;
	}

	useCustomTime(enabled: boolean): void {
		this.customTime = enabled ? currentLocalDateTime() : undefined;
	}

	async load(): Promise<void> {
		// A newer load (e.g. after an action) replaces an older one still in flight
		this.#loadController?.abort();
		const controller = new AbortController();
		this.#loadController = controller;

		const date = todayIsoDate();
		try {
			this.statuses = await api.get<JobStatus[]>(`/status?date=${date}`, {
				signal: controller.signal
			});
			this.failed = false;
		} catch {
			// Errors are already toasted; an aborted load is followed by a newer one
			if (!controller.signal.aborted) this.failed = true;
		} finally {
			if (!controller.signal.aborted) this.loading = false;
		}
	}

	async run(action: WorkdayAction): Promise<void> {
		const job = this.selected;
		const time = this.#actionTime();
		if (!job || !time || this.pending) return;

		this.pending = true;
		try {
			await runAction(job.jobId, action, time);
			const message = actionDoneMessage(action);
			showSuccess(this.customTime ? `${message} at ${this.customTime.time}` : message);
			// A back-dated time is for one action; the next one happens now again
			this.customTime = undefined;
		} catch {
			// Already toasted by the client's error handler
		} finally {
			// Also after a failure: the state may have changed elsewhere (e.g. another device)
			await this.load();
			this.pending = false;
		}
	}

	#actionTime(): ActionTime | undefined {
		if (!this.customTime) return { workDate: todayIsoDate() };
		const at = toInstant(this.customTime);
		return at === undefined ? undefined : { workDate: this.customTime.date, at };
	}
}
