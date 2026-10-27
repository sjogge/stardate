<script lang="ts">
	import type { FlexAdjustment } from '$lib/api/types';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { todayIsoDate } from '$lib/format';
	import { draftFromAdjustment, toAdjustmentRequest, type AdjustmentDraft } from './adjustmentForm';
	import type { History } from './history.svelte';

	let { history }: { history: History } = $props();

	const id = $props.id();
	// Undefined once the component is gone, e.g. after leaving the page while a save is in flight
	let dialog: HTMLDialogElement | undefined;
	let confirmDelete: ConfirmDialog;
	let editing = $state<FlexAdjustment>();
	let draft = $state<AdjustmentDraft>(draftFromAdjustment(undefined, ''));
	let error = $state<string>();

	/** Edits `adjustment`, or adds a new one dated today */
	export function open(adjustment?: FlexAdjustment): void {
		editing = adjustment;
		draft = draftFromAdjustment(adjustment, todayIsoDate());
		error = undefined;
		dialog?.showModal();
	}

	async function submit(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		const result = toAdjustmentRequest(draft);
		if ('error' in result) {
			error = result.error;
			return;
		}
		error = undefined;
		if (await history.saveAdjustment(result.request, editing)) dialog?.close();
	}

	async function remove(): Promise<void> {
		if (editing && (await history.removeAdjustment(editing))) dialog?.close();
	}
</script>

<dialog
	bind:this={dialog}
	aria-labelledby="{id}-title"
	class="dialog preset-filled-surface-100-900 animate-dialog max-md:dialog-fullscreen"
>
	<!-- novalidate: toAdjustmentRequest explains what is missing in one alert -->
	<form class="flex w-full flex-col gap-4 md:w-md" novalidate onsubmit={submit}>
		<h2 id="{id}-title" class="h4">{editing ? 'Edit adjustment' : 'Adjust flex balance'}</h2>
		<p class="text-sm opacity-75">
			For time that is not in your registered days, e.g. hours carried over from an earlier system
			or overtime paid out.
		</p>

		<fieldset class="flex flex-wrap items-center gap-4">
			<legend class="label-text">Change</legend>
			<label class="flex items-center gap-2">
				<input type="radio" class="radio" value="add" bind:group={draft.direction} />
				<span>Add time</span>
			</label>
			<label class="flex items-center gap-2">
				<input type="radio" class="radio" value="take" bind:group={draft.direction} />
				<span>Take time</span>
			</label>
		</fieldset>

		<div class="grid grid-cols-2 gap-3">
			<label class="label">
				<span class="label-text">Hours</span>
				<input class="input" placeholder="2:30" required bind:value={draft.amount} />
			</label>
			<label class="label">
				<span class="label-text">Date</span>
				<input type="date" class="input" required bind:value={draft.adjustedOn} />
			</label>
		</div>

		<label class="label">
			<span class="label-text">Note</span>
			<textarea class="textarea" rows="2" maxlength="1000" bind:value={draft.note}></textarea>
		</label>

		{#if error}
			<p role="alert" class="card preset-tonal-error p-3">{error}</p>
		{/if}

		<div class="flex flex-wrap justify-end gap-2">
			{#if editing}
				<button
					type="button"
					class="mr-auto btn preset-tonal-error"
					disabled={history.pending}
					onclick={() => confirmDelete.open()}
				>
					Delete
				</button>
			{/if}
			<button type="button" class="btn preset-tonal" onclick={() => dialog?.close()}>Cancel</button>
			<button type="submit" class="btn preset-filled-primary-500" disabled={history.pending}>
				Save
			</button>
		</div>
	</form>
</dialog>

<ConfirmDialog
	bind:this={confirmDelete}
	title="Delete this adjustment?"
	message="The flex balance no longer includes it."
	confirmLabel="Delete"
	onconfirm={remove}
/>
