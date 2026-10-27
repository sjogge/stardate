import { api } from '$lib/api/client';
import type { Job } from '$lib/api/types';
import { showSuccess } from '$lib/toast';

/** State of the jobs screen: all jobs, active ones first (the API's order). */
export class Jobs {
	jobs = $state<Job[]>([]);
	/** True until the first load has finished */
	loading = $state(true);
	failed = $state(false);
	/** A change is being sent; its buttons are disabled */
	pending = $state(false);

	get active(): Job[] {
		return this.jobs.filter((job) => job.archivedAt === null);
	}

	get archived(): Job[] {
		return this.jobs.filter((job) => job.archivedAt !== null);
	}

	async load(): Promise<void> {
		try {
			this.jobs = await api.get<Job[]>('/jobs');
			this.failed = false;
		} catch {
			// Already toasted by the client's error handler
			this.failed = true;
		} finally {
			this.loading = false;
		}
	}

	create(name: string): Promise<boolean> {
		return this.#change(() => api.post('/jobs', { body: { name } }), `Added ${name}`);
	}

	rename(job: Job, name: string): Promise<boolean> {
		return this.#change(
			() => api.put(`/jobs/${job.id}`, { body: { name }, ifMatch: job.version }),
			`Renamed to ${name}`
		);
	}

	archive(job: Job): Promise<boolean> {
		return this.#change(() => api.post(`/jobs/${job.id}/archive`), `Archived ${job.name}`);
	}

	remove(job: Job): Promise<boolean> {
		return this.#change(
			() => api.delete(`/jobs/${job.id}`, { ifMatch: job.version }),
			`Deleted ${job.name}`
		);
	}

	/** Sends a change and reloads the list; resolves to whether the change went through */
	async #change(send: () => Promise<unknown>, successMessage: string): Promise<boolean> {
		if (this.pending) return false;
		this.pending = true;
		try {
			await send();
			showSuccess(successMessage);
			return true;
		} catch {
			// Already toasted; e.g. a duplicate name (409) or a stale version (412)
			return false;
		} finally {
			// Also after a failure, so a stale version is replaced by the current one
			await this.load();
			this.pending = false;
		}
	}
}
