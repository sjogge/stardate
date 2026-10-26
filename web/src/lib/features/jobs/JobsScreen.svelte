<script lang="ts">
	import JobItem from './JobItem.svelte';
	import JobNameForm from './JobNameForm.svelte';
	import { Jobs } from './jobs.svelte';

	const jobs = new Jobs();

	$effect(() => {
		jobs.load();
	});
</script>

<div class="mx-auto flex max-w-md flex-col gap-4 p-4">
	<h1 class="h2">Jobs</h1>

	{#if jobs.loading}
		<div class="h-24 placeholder animate-pulse" aria-label="Loading"></div>
	{:else if jobs.failed && jobs.jobs.length === 0}
		<div class="space-y-3 card preset-tonal-error p-4">
			<p>The jobs could not be loaded.</p>
			<button type="button" class="btn preset-filled" onclick={() => jobs.load()}>
				Try again
			</button>
		</div>
	{:else}
		<section class="space-y-2">
			<h2 class="h4">Active</h2>
			{#if jobs.active.length === 0}
				<p class="opacity-75">No active jobs. Add one to start tracking time.</p>
			{:else}
				<ul class="space-y-2">
					{#each jobs.active as job (job.id)}
						<JobItem {job} {jobs} />
					{/each}
				</ul>
			{/if}
			<!-- Below the list: toasts appear at the top and would cover a form there -->
			<JobNameForm
				submitLabel="Add"
				pending={jobs.pending}
				onsubmit={(name) => jobs.create(name)}
			/>
		</section>

		{#if jobs.archived.length > 0}
			<section class="space-y-2">
				<h2 class="h4">Archived</h2>
				<ul class="space-y-2">
					{#each jobs.archived as job (job.id)}
						<JobItem {job} {jobs} />
					{/each}
				</ul>
			</section>
		{/if}
	{/if}
</div>
