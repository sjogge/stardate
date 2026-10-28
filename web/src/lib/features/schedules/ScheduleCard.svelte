<script lang="ts">
	import { Copy, Pencil, Repeat, Trash2 } from '@lucide/svelte';
	import type { Schedule } from '$lib/api/types';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { formatClockTime, formatDuration, formatWeekday } from '$lib/format';
	import type { Schedules } from './schedules.svelte';

	let {
		schedule,
		schedules,
		onedit,
		onduplicate
	}: {
		schedule: Schedule;
		schedules: Schedules;
		onedit: (schedule: Schedule) => void;
		onduplicate: (schedule: Schedule) => void;
	} = $props();

	let confirmDelete: ConfirmDialog;
	const next = $derived(schedules.nameOf(schedule.nextScheduleId));
</script>

<li class="space-y-3 card preset-filled-surface-100-900 p-4">
	<div class="flex items-center gap-2">
		<h2 class="grow h5">{schedule.name}</h2>
		<button
			type="button"
			class="btn-icon preset-tonal"
			aria-label="Edit {schedule.name}"
			title="Edit"
			disabled={schedules.pending}
			onclick={() => onedit(schedule)}
		>
			<Pencil class="size-4" aria-hidden="true" />
		</button>
		<button
			type="button"
			class="btn-icon preset-tonal"
			aria-label="Duplicate {schedule.name}"
			title="Duplicate"
			disabled={schedules.pending}
			onclick={() => onduplicate(schedule)}
		>
			<Copy class="size-4" aria-hidden="true" />
		</button>
		<button
			type="button"
			class="btn-icon preset-tonal-error"
			aria-label="Delete {schedule.name}"
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
	{#if next}
		<p class="flex items-center gap-1 text-sm">
			<Repeat class="size-4" aria-hidden="true" />
			<span>Next week: {next}</span>
		</p>
	{/if}

	{#if schedule.days.length === 0}
		<p class="text-sm opacity-75">No work expected on any day.</p>
	{:else}
		<!-- On narrow phones with am/pm times the table scrolls instead of the page -->
		<div class="overflow-x-auto">
			<table class="table text-sm">
				<thead>
					<tr>
						<th scope="col">Day</th>
						<th scope="col">Shift</th>
						<th scope="col">Work</th>
						<th scope="col">Lunch / breaks</th>
					</tr>
				</thead>
				<tbody class="tabular-nums">
					{#each schedule.days as day (day.weekday)}
						<tr>
							<th scope="row">{formatWeekday(day.weekday)}</th>
							<td class="whitespace-nowrap">
								{formatClockTime(day.start)}–{formatClockTime(day.end)}
							</td>
							<td>{formatDuration(day.workMinutes)}</td>
							<td class="whitespace-nowrap">
								{formatDuration(day.lunchMinutes)} / {formatDuration(day.breakMinutes)}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		{#if schedule.days.length < 7}
			<p class="text-sm opacity-75">Other days: no work expected.</p>
		{/if}
	{/if}
</li>

<ConfirmDialog
	bind:this={confirmDelete}
	title="Delete {schedule.name}?"
	message="Days already registered on it keep their expected times. A schedule that rotates into it no longer has a next week."
	confirmLabel="Delete"
	onconfirm={() => schedules.remove(schedule)}
/>
