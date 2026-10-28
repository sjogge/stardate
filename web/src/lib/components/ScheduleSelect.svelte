<script lang="ts">
	import type { Schedule } from '$lib/api/types';

	let {
		schedules,
		value,
		suggestedId,
		onchange
	}: {
		schedules: Schedule[];
		value: number | undefined;
		/** Marked in the list, so a changed choice can be set back to it */
		suggestedId?: number | null;
		onchange: (scheduleId: number) => void;
	} = $props();

	const id = $props.id();
</script>

<!-- Linked by id rather than wrapped, so the select's name is "Schedule" without the chosen option -->
<div class="label">
	<label for={id} class="label-text">Schedule</label>
	<select
		{id}
		class="select"
		value={value ?? ''}
		onchange={(event) => onchange(Number(event.currentTarget.value))}
	>
		{#if value === undefined}
			<option value="" disabled>Choose a schedule</option>
		{/if}
		{#each schedules as schedule (schedule.id)}
			<option value={schedule.id}>
				{schedule.name}{schedule.id === suggestedId ? ' (suggested)' : ''}
			</option>
		{/each}
	</select>
</div>
