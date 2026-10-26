<script lang="ts">
	import { todayIsoDate } from '$lib/format';
	import type { Workday } from './workday.svelte';

	let { workday }: { workday: Workday } = $props();
</script>

<div class="space-y-3">
	<label class="flex items-center gap-2">
		<input
			type="checkbox"
			class="checkbox"
			checked={workday.customTime !== undefined}
			onchange={(event) => workday.useCustomTime(event.currentTarget.checked)}
		/>
		<span>Use a different time</span>
	</label>

	{#if workday.customTime}
		<div class="grid grid-cols-2 gap-3">
			<label class="label">
				<span class="label-text">Date</span>
				<input
					type="date"
					class="input"
					required
					max={todayIsoDate()}
					bind:value={workday.customTime.date}
				/>
			</label>
			<label class="label">
				<span class="label-text">Time</span>
				<input type="time" class="input" required bind:value={workday.customTime.time} />
			</label>
		</div>
		<p class="text-sm opacity-75">
			The next action uses this time, e.g. when you forgot to clock in. A clock-in is registered on
			this date.
		</p>
	{/if}
</div>
