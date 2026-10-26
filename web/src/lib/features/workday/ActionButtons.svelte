<script lang="ts">
	import { Coffee, DoorOpen, LogIn, LogOut, Play, Utensils } from '@lucide/svelte';
	import type { Component } from 'svelte';
	import { actionLabel, type ActionChoice, type WorkdayAction } from './actions';

	let {
		choice,
		pending,
		onrun
	}: { choice: ActionChoice; pending: boolean; onrun: (action: WorkdayAction) => void } = $props();

	function iconFor(action: WorkdayAction): Component {
		switch (action.type) {
			case 'clockIn':
				return LogIn;
			case 'clockOut':
				return LogOut;
			case 'endPause':
				return Play;
			case 'startPause':
				return { lunch: Utensils, break: Coffee, other: DoorOpen }[action.pause];
		}
	}
</script>

{#snippet actionButton(action: WorkdayAction, classes: string)}
	{@const Icon = iconFor(action)}
	<button
		type="button"
		class={classes}
		disabled={pending}
		aria-busy={pending}
		onclick={() => onrun(action)}
	>
		<Icon aria-hidden="true" />
		<span>{actionLabel(action)}</span>
	</button>
{/snippet}

<div class="flex flex-col gap-3">
	{#if choice.primary}
		{@render actionButton(choice.primary, 'btn btn-lg preset-filled-primary-500 w-full py-5')}
	{/if}
	{#if choice.secondary.length > 0}
		<div class="grid grid-cols-2 gap-3">
			{#each choice.secondary as action (actionLabel(action))}
				{@render actionButton(action, 'btn preset-tonal w-full')}
			{/each}
		</div>
	{/if}
</div>
