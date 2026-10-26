<script lang="ts">
	import type { JobStatus } from '$lib/api/types';
	import {
		formatBalance,
		formatDuration,
		formatShortDate,
		formatTime,
		todayIsoDate
	} from '$lib/format';
	import { PAUSE_NAMES } from './actions';

	let { status }: { status: JobStatus } = $props();

	const today = $derived(status.today);

	const headline = $derived.by(() => {
		if (status.runningPause) {
			const { kind, startedAt } = status.runningPause;
			return `On ${PAUSE_NAMES[kind]} since ${formatTime(startedAt)}`;
		}
		if (!today) return 'Not clocked in';
		if (today.kind === 'flex') return 'Flex day';
		if (!today.startedAt) return 'Not clocked in';
		if (!today.endedAt) {
			// A day still open from an earlier date (e.g. a night shift) says which day it is
			const day = today.workDate === todayIsoDate() ? '' : `${formatShortDate(today.workDate)} `;
			return `Working since ${day}${formatTime(today.startedAt)}`;
		}
		return `Done: ${formatTime(today.startedAt)}–${formatTime(today.endedAt)}`;
	});
</script>

<section class="space-y-4 card preset-filled-surface-100-900 p-4" aria-live="polite">
	<p class="h4">{headline}</p>

	<dl class="grid grid-cols-2 gap-4">
		{#if today?.kind === 'work'}
			<div>
				<dt class="label-text">Worked</dt>
				<dd class="text-xl tabular-nums">{formatDuration(today.totals.workedMinutes)}</dd>
			</div>
			{#if today.totals.leaveAt}
				<div>
					<dt class="label-text">Leave at</dt>
					<dd class="text-xl tabular-nums">{formatTime(today.totals.leaveAt)}</dd>
				</div>
			{/if}
		{/if}
		{#if today}
			<div>
				<dt class="label-text">Today's balance</dt>
				<dd class="text-xl tabular-nums">{formatBalance(today.totals.balanceMinutes)}</dd>
			</div>
		{/if}
		<div>
			<dt class="label-text">Flex balance</dt>
			<dd class="text-xl tabular-nums">{formatBalance(status.flexBalanceMinutes)}</dd>
		</div>
	</dl>
</section>
