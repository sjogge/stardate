import { api } from '$lib/api/client';
import type { Job, Schedule } from '$lib/api/types';
import { showSuccess } from '$lib/toast';
import type { ScheduleRequest } from './scheduleForm';

/** State of one job's schedule page: the job and its schedule versions. */
export class Schedules {
	readonly jobId: number;
	job = $state<Job>();
	/** Oldest first, as the API returns them */
	schedules = $state<Schedule[]>([]);
	/** True until the first load has finished */
	loading = $state(true);
	failed = $state(false);
	/** A change is being sent; its buttons are disabled */
	pending = $state(false);

	constructor(jobId: number) {
		this.jobId = jobId;
	}

	/** The most recent version first: that is the one people change */
	get newestFirst(): Schedule[] {
		return this.schedules.toReversed();
	}

	get latest(): Schedule | undefined {
		return this.schedules.at(-1);
	}

	async load(): Promise<void> {
		const path = `/jobs/${this.jobId}`;
		try {
			[this.job, this.schedules] = await Promise.all([
				api.get<Job>(path),
				api.get<Schedule[]>(`${path}/schedules`)
			]);
			this.failed = false;
		} catch {
			// Already toasted by the client's error handler
			this.failed = true;
		} finally {
			this.loading = false;
		}
	}

	save(request: ScheduleRequest, existing?: Schedule): Promise<boolean> {
		const path = `/jobs/${this.jobId}/schedules`;
		return this.#change(
			() =>
				existing
					? api.put(`${path}/${existing.id}`, { body: request, ifMatch: existing.version })
					: api.post(path, { body: request }),
			existing ? 'Schedule saved' : 'Schedule added'
		);
	}

	remove(schedule: Schedule): Promise<boolean> {
		return this.#change(
			() =>
				api.delete(`/jobs/${this.jobId}/schedules/${schedule.id}`, {
					ifMatch: schedule.version
				}),
			'Schedule deleted'
		);
	}

	/** Sends a change and reloads; resolves to whether the change went through */
	async #change(send: () => Promise<unknown>, successMessage: string): Promise<boolean> {
		if (this.pending) return false;
		this.pending = true;
		try {
			await send();
			showSuccess(successMessage);
			return true;
		} catch {
			// Already toasted; e.g. a second schedule on the same date (409) or a stale version (412)
			return false;
		} finally {
			await this.load();
			this.pending = false;
		}
	}
}
