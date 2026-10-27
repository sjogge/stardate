<script lang="ts">
	// Mirrors the backend's limit (jobs.rs) so the input stops there; the backend still checks
	const MAX_NAME_LENGTH = 100;

	let {
		initialName = '',
		submitLabel,
		pending,
		onsubmit,
		oncancel
	}: {
		initialName?: string;
		submitLabel: string;
		pending: boolean;
		/** Resolves to whether the name was saved; the field is cleared when it was */
		onsubmit: (name: string) => Promise<boolean>;
		oncancel?: () => void;
	} = $props();

	// Starts from the current name once; later edits stay local until saved
	// svelte-ignore state_referenced_locally
	let name = $state(initialName);
	const trimmed = $derived(name.trim());

	async function submit(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		if (trimmed === '') return;
		if (await onsubmit(trimmed)) name = '';
	}
</script>

<form class="flex gap-2" onsubmit={submit}>
	<label class="grow">
		<span class="sr-only">Job name</span>
		<input
			class="input"
			placeholder="Job name"
			required
			maxlength={MAX_NAME_LENGTH}
			bind:value={name}
		/>
	</label>
	<button type="submit" class="btn preset-filled-primary-500" disabled={pending || trimmed === ''}>
		{submitLabel}
	</button>
	{#if oncancel}
		<button type="button" class="btn preset-tonal" onclick={oncancel}>Cancel</button>
	{/if}
</form>
