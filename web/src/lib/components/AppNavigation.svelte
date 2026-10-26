<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { Briefcase, Clock, History, Settings } from '@lucide/svelte';
	import { Navigation } from '@skeletonlabs/skeleton-svelte';
	import { MediaQuery } from 'svelte/reactivity';

	// Tailwind's `md` breakpoint: a bottom bar on phones, a side rail from tablets up
	const wideScreen = new MediaQuery('(min-width: 48rem)');

	/** The link's page or a page below it, e.g. a job's schedules under Jobs */
	function isCurrent(href: string): boolean {
		const path = page.url.pathname;
		return path === href || (href !== resolve('/') && path.startsWith(`${href}/`));
	}

	const links = [
		{ label: 'Today', href: resolve('/'), icon: Clock },
		{ label: 'History', href: resolve('/history'), icon: History },
		{ label: 'Jobs', href: resolve('/jobs'), icon: Briefcase },
		{ label: 'Settings', href: resolve('/settings'), icon: Settings }
	];
</script>

<Navigation layout={wideScreen.current ? 'rail' : 'bar'}>
	<Navigation.Menu class={wideScreen.current ? '' : 'grid grid-cols-4 gap-2'}>
		{#each links as link (link.href)}
			{@const Icon = link.icon}
			<Navigation.TriggerAnchor
				href={link.href}
				aria-current={isCurrent(link.href) ? 'page' : undefined}
				class="aria-[current=page]:preset-tonal-primary"
			>
				<Icon class="size-5" aria-hidden="true" />
				<Navigation.TriggerText>{link.label}</Navigation.TriggerText>
			</Navigation.TriggerAnchor>
		{/each}
	</Navigation.Menu>
</Navigation>
