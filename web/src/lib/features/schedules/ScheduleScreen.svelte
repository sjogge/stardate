<script lang="ts">
	import { resolve } from '$app/paths';
	import { ArrowLeft, Plus } from '@lucide/svelte';
	import ScheduleCard from './ScheduleCard.svelte';
	import ScheduleDialog from './ScheduleDialog.svelte';
	import { Schedules } from './schedules.svelte';

	let { jobId }: { jobId: number } = $props();

	// The route recreates this component for another job ({#key}), so the id is read once
	// svelte-ignore state_referenced_locally
	const schedules = new Schedules(jobId);
	let dialog: ScheduleDialog;

	$effect(() => {
		schedules.load();
	});
</script>

<div class="mx-auto flex max-w-md flex-col gap-4 p-4">
	<a class="flex items-center gap-1 self-start anchor" href={resolve('/jobs')}>
		<ArrowLeft class="size-4" aria-hidden="true" />
		<span>Jobs</span>
	</a>
	<div>
		<h1 class="h2">Schedules</h1>
		{#if schedules.job}
			<p class="opacity-75">{schedules.job.name}</p>
		{/if}
	</div>

	{#if schedules.loading}
		<div class="h-32 placeholder animate-pulse" aria-label="Loading"></div>
	{:else if schedules.failed && !schedules.job}
		<div class="space-y-3 card preset-tonal-error p-4">
			<p>The schedules could not be loaded.</p>
			<button type="button" class="btn preset-filled" onclick={() => schedules.load()}>
				Try again
			</button>
		</div>
	{:else}
		<p class="text-sm opacity-75">
			A schedule is a week of planned shifts. You pick one for every day you register, so add one
			per kind of week (e.g. Daytime and Nights), and link them with "Next week" if they rotate.
			Changing a schedule never changes days already registered.
		</p>
		<button
			type="button"
			class="btn self-start preset-filled-primary-500"
			disabled={schedules.pending}
			onclick={() => dialog.open()}
		>
			<Plus class="size-4" aria-hidden="true" />
			<span>Add schedule</span>
		</button>

		{#if schedules.schedules.length === 0}
			<p class="card preset-tonal p-4">No schedule yet: add one before registering days.</p>
		{:else}
			<ul class="space-y-3">
				{#each schedules.schedules as schedule (schedule.id)}
					<ScheduleCard
						{schedule}
						{schedules}
						onedit={(selected) => dialog.open(selected)}
						onduplicate={(original) => dialog.open(undefined, original)}
					/>
				{/each}
			</ul>
		{/if}
	{/if}
</div>

<ScheduleDialog bind:this={dialog} {schedules} />
