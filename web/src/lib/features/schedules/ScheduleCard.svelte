<script lang="ts">
	import { Pencil, Trash2 } from '@lucide/svelte';
	import type { Schedule } from '$lib/api/types';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { formatDate, formatDuration, formatWeekday } from '$lib/format';
	import type { Schedules } from './schedules.svelte';

	let {
		schedule,
		schedules,
		onedit
	}: { schedule: Schedule; schedules: Schedules; onedit: (schedule: Schedule) => void } = $props();

	let confirmDelete: ConfirmDialog;
	const from = $derived(formatDate(schedule.validFrom));
</script>

<li class="space-y-3 card preset-filled-surface-100-900 p-4">
	<div class="flex items-center gap-2">
		<h2 class="grow h5">From {from}</h2>
		<button
			type="button"
			class="btn-icon preset-tonal"
			aria-label="Edit the schedule from {from}"
			title="Edit"
			disabled={schedules.pending}
			onclick={() => onedit(schedule)}
		>
			<Pencil class="size-4" aria-hidden="true" />
		</button>
		<button
			type="button"
			class="btn-icon preset-tonal-error"
			aria-label="Delete the schedule from {from}"
			title="Delete"
			disabled={schedules.pending}
			onclick={() => confirmDelete.open()}
		>
			<Trash2 class="size-4" aria-hidden="true" />
		</button>
	</div>

	<p class="text-sm">
		Lunch {schedule.lunchPaid ? 'paid' : 'unpaid'} · Breaks {schedule.breaksPaid
			? 'paid'
			: 'unpaid'}
	</p>

	{#if schedule.days.length === 0}
		<p class="text-sm opacity-75">No work expected on any day.</p>
	{:else}
		<table class="table">
			<thead>
				<tr>
					<th scope="col">Day</th>
					<th scope="col">Work</th>
					<th scope="col">Lunch</th>
					<th scope="col">Breaks</th>
				</tr>
			</thead>
			<tbody class="tabular-nums">
				{#each schedule.days as day (day.weekday)}
					<tr>
						<th scope="row">{formatWeekday(day.weekday)}</th>
						<td>{formatDuration(day.workMinutes)}</td>
						<td>{formatDuration(day.lunchMinutes)}</td>
						<td>{formatDuration(day.breakMinutes)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
		{#if schedule.days.length < 7}
			<p class="text-sm opacity-75">Other days: no work expected.</p>
		{/if}
	{/if}
</li>

<ConfirmDialog
	bind:this={confirmDelete}
	title="Delete the schedule from {from}?"
	message="Days already registered keep their expected times. Days registered later use the previous schedule instead."
	confirmLabel="Delete"
	onconfirm={() => schedules.remove(schedule)}
/>
