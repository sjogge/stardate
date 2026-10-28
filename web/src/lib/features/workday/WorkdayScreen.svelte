<script lang="ts">
	import { resolve } from '$app/paths';
	import ScheduleSelect from '$lib/components/ScheduleSelect.svelte';
	import ActionButtons from './ActionButtons.svelte';
	import CustomTimeFields from './CustomTimeFields.svelte';
	import StatusSummary from './StatusSummary.svelte';
	import { chooseActions } from './actions';
	import { Workday } from './workday.svelte';

	// Worked time and leave-at move on while the page is open; the server recomputes them
	const REFRESH_INTERVAL_MS = 60_000;

	const workday = new Workday();
	const choice = $derived(workday.selected ? chooseActions(workday.selected) : undefined);
	/** Clocking in registers the day on a schedule, so the selector is shown only then */
	const canClockIn = $derived(workday.selected?.availableActions.includes('clockIn') ?? false);

	$effect(() => {
		const jobId = workday.selected?.jobId;
		if (canClockIn && jobId !== undefined) workday.loadSchedules(jobId);
	});

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
		<p class="card preset-tonal p-4">
			No active jobs yet. <a class="anchor" href={resolve('/jobs')}>Add a job</a> to start tracking time.
		</p>
	{:else}
		{#if workday.statuses.length > 1}
			<label class="label">
				<span class="label-text">Job</span>
				<select
					class="select"
					value={workday.selected.jobId}
					onchange={(event) => workday.selectJob(Number(event.currentTarget.value))}
				>
					{#each workday.statuses as status (status.jobId)}
						<option value={status.jobId}>{status.jobName}</option>
					{/each}
				</select>
			</label>
		{/if}

		<StatusSummary status={workday.selected} />
		{#if !workday.selected.today && workday.selected.availableActions.length === 0}
			<!-- The backend offers no clock-in while the job has no schedules -->
			<p class="card preset-tonal-warning p-4">
				Every day is registered on a schedule, so you cannot clock in yet.
				<a
					class="anchor"
					href={resolve('/jobs/[jobId=integer]', { jobId: String(workday.selected.jobId) })}
				>
					Add a schedule
				</a>
				for {workday.selected.jobName} first.
			</p>
		{/if}
		{#if canClockIn}
			<ScheduleSelect
				schedules={workday.schedules}
				value={workday.scheduleId}
				suggestedId={workday.selected.suggestedScheduleId}
				onchange={(scheduleId) => (workday.chosenScheduleId = scheduleId)}
			/>
		{/if}
		<ActionButtons
			{choice}
			pending={workday.pending}
			disabled={!workday.timeIsValid || (canClockIn && workday.scheduleId === undefined)}
			onrun={(action) => workday.run(action)}
		/>
		{#if choice.primary}
			<CustomTimeFields {workday} />
		{/if}
	{/if}
</div>
