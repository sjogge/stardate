<script lang="ts">
	import { SegmentedControl } from '@skeletonlabs/skeleton-svelte';
	import { MODES, THEMES, appearance, type Mode, type ThemeName } from '$lib/theme.svelte';

	const modeLabels: Record<Mode, string> = { light: 'Light', dark: 'Dark', system: 'System' };

	// Narrow the plain strings the controls hand back to the allowed values
	function isTheme(value: string): value is ThemeName {
		return THEMES.some((theme) => theme === value);
	}

	function isMode(value: string | null): value is Mode {
		return MODES.some((mode) => mode === value);
	}
</script>

<section class="space-y-4 card preset-filled-surface-100-900 p-4">
	<h2 class="h4">Appearance</h2>

	<label class="label">
		<span class="label-text">Theme</span>
		<select
			class="select capitalize"
			value={appearance.theme}
			onchange={(event) => {
				const { value } = event.currentTarget;
				if (isTheme(value)) appearance.setTheme(value);
			}}
		>
			{#each THEMES as theme (theme)}
				<option value={theme}>{theme}</option>
			{/each}
		</select>
	</label>

	<SegmentedControl
		value={appearance.mode}
		onValueChange={({ value }) => {
			if (isMode(value)) appearance.setMode(value);
		}}
	>
		<SegmentedControl.Label class="label-text">Mode</SegmentedControl.Label>
		<SegmentedControl.Control>
			<SegmentedControl.Indicator />
			{#each MODES as mode (mode)}
				<SegmentedControl.Item value={mode}>
					<SegmentedControl.ItemText>{modeLabels[mode]}</SegmentedControl.ItemText>
					<SegmentedControl.ItemHiddenInput />
				</SegmentedControl.Item>
			{/each}
		</SegmentedControl.Control>
	</SegmentedControl>
</section>
