<script lang="ts">
	import ActionButtons from './ActionButtons.svelte';
	import CustomTimeFields from './CustomTimeFields.svelte';
	import StatusSummary from './StatusSummary.svelte';
	import { chooseActions } from './actions';
	import { Workday } from './workday.svelte';

	// Worked time and leave-at move on while the page is open; the server recomputes them
	const REFRESH_INTERVAL_MS = 60_000;

	const workday = new Workday();
	const choice = $derived(workday.selected ? chooseActions(workday.selected) : undefined);

	$effect(() => {
		workday.load();

		const refreshIfVisible = () => {
			if (document.visibilityState === 'visible') workday.load();
		};
		const timer = setInterval(refreshIfVisible, REFRESH_INTERVAL_MS);
		document.addEventListener('visibilitychange', refreshIfVisible);
		return () => {
			clearInterval(timer);
			document.removeEventListener('visibilitychange', refreshIfVisible);
		};
	});
</script>

<div class="mx-auto flex max-w-md flex-col gap-4 p-4">
	<h1 class="h2">Today</h1>

	{#if workday.loading}
		<div class="h-32 placeholder animate-pulse" aria-label="Loading"></div>
		<div class="h-16 placeholder animate-pulse"></div>
	{:else if workday.failed && workday.statuses.length === 0}
		<div class="space-y-3 card preset-tonal-error p-4">
			<p>The status could not be loaded.</p>
			<button type="button" class="btn preset-filled" onclick={() => workday.load()}>
				Try again
			</button>
		</div>
	{:else if !workday.selected || !choice}
		<p class="card preset-tonal p-4">No active jobs yet.</p>
	{:else}
		{#if workday.statuses.length > 1}
			<label class="label">
				<span class="label-text">Job</span>
				<select
					class="select"
					value={workday.selected.jobId}
					onchange={(event) => (workday.selectedJobId = Number(event.currentTarget.value))}
				>
					{#each workday.statuses as status (status.jobId)}
						<option value={status.jobId}>{status.jobName}</option>
					{/each}
				</select>
			</label>
		{/if}

		<StatusSummary status={workday.selected} />
		<ActionButtons
			{choice}
			pending={workday.pending}
			disabled={!workday.timeIsValid}
			onrun={(action) => workday.run(action)}
		/>
		{#if choice.primary}
			<CustomTimeFields {workday} />
		{/if}
	{/if}
</div>
