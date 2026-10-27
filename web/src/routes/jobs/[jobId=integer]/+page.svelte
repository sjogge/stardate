<script lang="ts">
	import { page } from '$app/state';
	import ScheduleScreen from '$lib/features/schedules/ScheduleScreen.svelte';

	// The `integer` matcher guarantees a positive whole number
	const jobId = $derived(Number(page.params.jobId));
</script>

<svelte:head><title>Schedules · Stardate</title></svelte:head>

{#key jobId}
	<ScheduleScreen {jobId} />
{/key}
