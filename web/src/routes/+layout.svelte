<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { Toast } from '@skeletonlabs/skeleton-svelte';
	import { setErrorHandler } from '$lib/api/client';
	import { showError, toaster } from '$lib/toast';

	let { children } = $props();

	setErrorHandler((error) => showError(error.message));
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
{@render children()}

<Toast.Group {toaster}>
	{#snippet children(toast)}
		<Toast {toast}>
			<Toast.Message>
				<Toast.Title>{toast.title}</Toast.Title>
				{#if toast.description}
					<Toast.Description>{toast.description}</Toast.Description>
				{/if}
			</Toast.Message>
			<Toast.CloseTrigger />
		</Toast>
	{/snippet}
</Toast.Group>
