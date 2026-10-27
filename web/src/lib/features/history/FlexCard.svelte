<script lang="ts">
	import { Pencil, Plus } from '@lucide/svelte';
	import type { FlexAdjustment, FlexBalance } from '$lib/api/types';
	import { formatBalance, formatDate } from '$lib/format';

	let {
		flex,
		adjustments,
		pending,
		onadd,
		onedit
	}: {
		flex: FlexBalance;
		/** Newest first */
		adjustments: FlexAdjustment[];
		pending: boolean;
		onadd: () => void;
		onedit: (adjustment: FlexAdjustment) => void;
	} = $props();
</script>

<section class="space-y-3 card preset-filled-surface-100-900 p-4" aria-labelledby="flex-title">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<div>
			<h2 id="flex-title" class="label-text">Flex balance</h2>
			<p class="text-2xl tabular-nums">{formatBalance(flex.balanceMinutes)}</p>
		</div>
		<button type="button" class="btn preset-tonal" disabled={pending} onclick={onadd}>
			<Plus class="size-4" aria-hidden="true" />
			<span>Adjust</span>
		</button>
	</div>
	<p class="text-sm opacity-75">
		Days {formatBalance(flex.daysMinutes)} · Adjustments {formatBalance(flex.adjustmentsMinutes)}.
		All time, closed days only: a day still open counts once you clock out.
	</p>

	{#if adjustments.length > 0}
		<details>
			<summary class="cursor-pointer text-sm">Adjustments ({adjustments.length})</summary>
			<ul class="mt-2 space-y-2">
				{#each adjustments as adjustment (adjustment.id)}
					{@const date = formatDate(adjustment.adjustedOn)}
					<li class="flex items-center gap-2">
						<span class="w-28 shrink-0 text-sm">{date}</span>
						<span class="w-16 shrink-0 tabular-nums">{formatBalance(adjustment.minutes)}</span>
						<span class="grow truncate text-sm opacity-75">{adjustment.note ?? ''}</span>
						<button
							type="button"
							class="btn-icon preset-tonal"
							aria-label="Edit the adjustment of {date}"
							title="Edit"
							disabled={pending}
							onclick={() => onedit(adjustment)}
						>
							<Pencil class="size-4" aria-hidden="true" />
						</button>
					</li>
				{/each}
			</ul>
		</details>
	{/if}
</section>
