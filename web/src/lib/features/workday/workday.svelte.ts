import { api } from '$lib/api/client';
import type { JobStatus } from '$lib/api/types';
import { todayIsoDate } from '$lib/format';
import { showSuccess } from '$lib/toast';
import { actionDoneMessage, runAction, type WorkdayAction } from './actions';

/** State of the workday screen: every active job's status, and the job being shown. */
export class Workday {
	statuses = $state<JobStatus[]>([]);
	selectedJobId = $state<number>();
	/** True until the first load has finished */
	loading = $state(true);
	failed = $state(false);
	/** An action is being sent; its buttons are disabled */
	pending = $state(false);

	#loadController: AbortController | undefined;

	/** The chosen job; until one is chosen, the job you are clocked in on, else the first */
	get selected(): JobStatus | undefined {
		return (
			this.statuses.find((status) => status.jobId === this.selectedJobId) ??
			this.statuses.find((status) => status.clockedIn) ??
			this.statuses[0]
		);
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
		if (!job || this.pending) return;

		this.pending = true;
		try {
			await runAction(job.jobId, action, todayIsoDate());
			showSuccess(actionDoneMessage(action));
		} catch {
			// Already toasted by the client's error handler
		} finally {
			// Also after a failure: the state may have changed elsewhere (e.g. another device)
			await this.load();
			this.pending = false;
		}
	}
}
