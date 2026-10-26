<script lang="ts">
	import { ChevronLeft, ChevronRight } from '@lucide/svelte';
	import { formatBalance, formatDuration, formatMonth } from '$lib/format';
	import DayList from './DayList.svelte';
	import { History } from './history.svelte';

	const history = new History();
	const monthName = $derived(formatMonth(history.month.year, history.month.month));

	$effect(() => {
		history.init();
	});
</script>

<div class="mx-auto flex max-w-3xl flex-col gap-4 p-4">
	<h1 class="h2">History</h1>

	{#if history.jobs.length > 1}
		<label class="label md:max-w-xs">
			<span class="label-text">Job</span>
			<select
				class="select"
				value={history.jobId}
				onchange={(event) => history.selectJob(Number(event.currentTarget.value))}
			>
				{#each history.jobs as job (job.id)}
					<option value={job.id}>{job.name}{job.archivedAt ? ' (archived)' : ''}</option>
				{/each}
			</select>
		</label>
	{/if}

	{#if history.jobId === undefined && !history.loading}
		{#if history.failed}
			<div class="space-y-3 card preset-tonal-error p-4">
				<p>The jobs could not be loaded.</p>
				<button type="button" class="btn preset-filled" onclick={() => history.init()}>
					Try again
				</button>
			</div>
		{:else}
			<p class="card preset-tonal p-4">No jobs yet, so there is no history to show.</p>
		{/if}
	{:else}
		<div class="flex items-center justify-between gap-2">
			<button
				type="button"
				class="btn-icon preset-tonal"
				aria-label="Previous month"
				onclick={() => history.showMonth(-1)}
			>
				<ChevronLeft class="size-5" aria-hidden="true" />
			</button>
			<h2 class="h4" aria-live="polite">{monthName}</h2>
			<button
				type="button"
				class="btn-icon preset-tonal"
				aria-label="Next month"
				disabled={history.isCurrentMonth}
				onclick={() => history.showMonth(1)}
			>
				<ChevronRight class="size-5" aria-hidden="true" />
			</button>
		</div>

		{#if history.loading}
			<div class="h-20 placeholder animate-pulse" aria-label="Loading"></div>
			<div class="h-40 placeholder animate-pulse"></div>
		{:else if history.failed}
			<div class="space-y-3 card preset-tonal-error p-4">
				<p>{monthName} could not be loaded.</p>
				<button type="button" class="btn preset-filled" onclick={() => history.load()}>
					Try again
				</button>
			</div>
		{:else}
			{#if history.totals}
				<section class="card preset-filled-surface-100-900 p-4" aria-label="Totals for {monthName}">
					<dl class="grid grid-cols-3 gap-4">
						<div>
							<dt class="label-text">Worked</dt>
							<dd class="text-xl tabular-nums">{formatDuration(history.totals.workedMinutes)}</dd>
						</div>
						<div>
							<dt class="label-text">Expected</dt>
							<dd class="text-xl tabular-nums">
								{formatDuration(history.totals.expectedWorkMinutes)}
							</dd>
						</div>
						<div>
							<dt class="label-text">Balance</dt>
							<dd class="text-xl tabular-nums">{formatBalance(history.totals.balanceMinutes)}</dd>
						</div>
					</dl>
					<p class="mt-2 text-sm opacity-75">
						Closed days and flex adjustments; a day still open counts once you clock out.
					</p>
				</section>
			{/if}

			{#if history.days.length === 0}
				<p class="card preset-tonal p-4">No days registered in {monthName}.</p>
			{:else}
				<DayList days={history.newestFirst} />
			{/if}
		{/if}
	{/if}
</div>
