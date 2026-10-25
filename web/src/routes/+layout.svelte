<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { Toast } from '@skeletonlabs/skeleton-svelte';
	import { setErrorHandler } from '$lib/api/client';
	import { showError, toaster } from '$lib/toast';
	import AppNavigation from '$lib/components/AppNavigation.svelte';
	import { applyAppearance } from '$lib/theme.svelte';

	let { children } = $props();

	setErrorHandler((error) => showError(error.message));
	$effect(applyAppearance);
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
<!-- Phones: content above a bottom bar. From md: a rail on the left. -->
<div class="grid h-dvh grid-rows-[1fr_auto] md:grid-cols-[auto_1fr] md:grid-rows-1">
	<main class="overflow-y-auto">
		{@render children()}
	</main>
	<div class="md:order-first">
		<AppNavigation />
	</div>
</div>

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
