<script lang="ts">
	import { Archive, Pencil, Trash2 } from '@lucide/svelte';
	import type { Job } from '$lib/api/types';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import JobNameForm from './JobNameForm.svelte';
	import type { Jobs } from './jobs.svelte';

	let { job, jobs }: { job: Job; jobs: Jobs } = $props();

	let editing = $state(false);
	let confirmArchive: ConfirmDialog;
	let confirmDelete: ConfirmDialog;

	async function rename(name: string): Promise<boolean> {
		const renamed = await jobs.rename(job, name);
		if (renamed) editing = false;
		return renamed;
	}
</script>

<li class="card preset-filled-surface-100-900 p-3">
	{#if editing}
		<JobNameForm
			initialName={job.name}
			submitLabel="Save"
			pending={jobs.pending}
			onsubmit={rename}
			oncancel={() => (editing = false)}
		/>
	{:else}
		<div class="flex items-center gap-2">
			<span class="grow truncate">{job.name}</span>
			{#if job.archivedAt === null}
				<button
					type="button"
					class="btn-icon preset-tonal"
					aria-label="Rename {job.name}"
					title="Rename"
					disabled={jobs.pending}
					onclick={() => (editing = true)}
				>
					<Pencil class="size-4" aria-hidden="true" />
				</button>
				<button
					type="button"
					class="btn-icon preset-tonal"
					aria-label="Archive {job.name}"
					title="Archive"
					disabled={jobs.pending}
					onclick={() => confirmArchive.open()}
				>
					<Archive class="size-4" aria-hidden="true" />
				</button>
			{:else}
				<button
					type="button"
					class="btn-icon preset-tonal-error"
					aria-label="Delete {job.name}"
					title="Delete"
					disabled={jobs.pending}
					onclick={() => confirmDelete.open()}
				>
					<Trash2 class="size-4" aria-hidden="true" />
				</button>
			{/if}
		</div>
	{/if}
</li>

<ConfirmDialog
	bind:this={confirmArchive}
	title="Archive {job.name}?"
	message="It no longer appears on the Today screen and takes no new days. Its history and flex balance are kept. An archived job cannot be made active again."
	confirmLabel="Archive"
	onconfirm={() => jobs.archive(job)}
/>
<ConfirmDialog
	bind:this={confirmDelete}
	title="Delete {job.name}?"
	message="This deletes the job with all its schedules, days and flex adjustments. It cannot be undone."
	confirmLabel="Delete"
	onconfirm={() => jobs.remove(job)}
/>
