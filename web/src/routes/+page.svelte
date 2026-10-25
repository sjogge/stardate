<script lang="ts">
	import { Clock } from '@lucide/svelte';
</script>

<main class="mx-auto flex max-w-md flex-col items-center gap-4 p-4">
	<h1 class="h1">Stardate</h1>
	<button type="button" class="btn preset-filled-primary-500">
		<Clock aria-hidden="true" />
		<span>Clock in</span>
	</button>
</main>
