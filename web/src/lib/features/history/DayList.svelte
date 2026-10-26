<script lang="ts">
	import type { Day } from '$lib/api/types';
	import { formatBalance, formatDuration, formatShortDate } from '$lib/format';
	import { dayTimes } from './dayText';

	let { days }: { days: Day[] } = $props();
</script>

<!-- A table from md up; a list of cards on phones, where five columns would not fit.
     The wrapper hides the table: Skeleton's `table` class sets its own display. -->
<div class="hidden md:block">
	<table class="table">
		<thead>
			<tr>
				<th scope="col">Date</th>
				<th scope="col">Times</th>
				<th scope="col">Worked</th>
				<th scope="col">Balance</th>
				<th scope="col">Note</th>
			</tr>
		</thead>
		<tbody>
			{#each days as day (day.id)}
				<tr>
					<th scope="row">{formatShortDate(day.workDate)}</th>
					<td class="tabular-nums">{dayTimes(day)}</td>
					<td class="tabular-nums">{formatDuration(day.totals.workedMinutes)}</td>
					<td class="tabular-nums">{formatBalance(day.totals.balanceMinutes)}</td>
					<td class="max-w-48 truncate">{day.note ?? ''}</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>

<ul class="space-y-2 md:hidden">
	{#each days as day (day.id)}
		<li class="card preset-filled-surface-100-900 p-3">
			<div class="flex items-baseline justify-between gap-2">
				<span class="font-bold">{formatShortDate(day.workDate)}</span>
				<span class="tabular-nums">{formatBalance(day.totals.balanceMinutes)}</span>
			</div>
			<div class="flex justify-between gap-2 text-sm opacity-75">
				<span class="tabular-nums">{dayTimes(day)}</span>
				<span class="tabular-nums">Worked {formatDuration(day.totals.workedMinutes)}</span>
			</div>
			{#if day.note}
				<p class="mt-1 truncate text-sm">{day.note}</p>
			{/if}
		</li>
	{/each}
</ul>
