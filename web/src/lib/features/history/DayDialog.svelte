<script lang="ts">
	import { Plus, X } from '@lucide/svelte';
	import type { Day, PauseKind } from '$lib/api/types';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { formatShortDate, todayIsoDate } from '$lib/format';
	import { PAUSE_NAMES } from '$lib/features/workday/actions';
	import { draftFromDay, emptyPause, toDayRequest, type DayDraft } from './dayForm';
	import type { History } from './history.svelte';

	let { history }: { history: History } = $props();

	const PAUSE_KINDS: PauseKind[] = ['lunch', 'break', 'other'];

	const id = $props.id();
	// Undefined once the component is gone, e.g. after leaving the page while a save is in flight
	let dialog: HTMLDialogElement | undefined;
	let confirmDelete: ConfirmDialog;
	let editing = $state<Day>();
	let draft = $state<DayDraft>(draftFromDay(undefined, ''));
	let error = $state<string>();

	/** Edits `day`, or registers a new day on `workDate` */
	export function open(day?: Day, workDate = todayIsoDate()): void {
		editing = day;
		draft = draftFromDay(day, workDate);
		error = undefined;
		dialog?.showModal();
	}

	async function submit(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		const result = toDayRequest(draft);
		// Caught here because the backend's answer (412) only says the day changed elsewhere
		const taken = !editing && history.days.some((day) => day.workDate === draft.workDate);
		if (taken) {
			error = `${formatShortDate(draft.workDate)} is already registered; edit that day instead.`;
			return;
		}
		if ('error' in result) {
			error = result.error;
			return;
		}
		error = undefined;
		if (await history.save(draft.workDate, result.request, editing)) dialog?.close();
	}

	async function remove(): Promise<void> {
		if (editing && (await history.remove(editing))) dialog?.close();
	}
</script>

<!-- Full screen on phones, a centered dialog from md; the form scrolls inside either -->
<dialog
	bind:this={dialog}
	aria-labelledby="{id}-title"
	class="dialog preset-filled-surface-100-900 animate-dialog max-md:dialog-fullscreen md:max-h-[90dvh]"
>
	<!-- novalidate: toDayRequest explains what is missing in one alert instead of browser bubbles -->
	<form
		class="flex min-h-0 w-full flex-col gap-4 overflow-y-auto md:w-lg"
		novalidate
		onsubmit={submit}
	>
		<h2 id="{id}-title" class="h4">
			{editing ? `Edit ${formatShortDate(editing.workDate)}` : 'Add a day'}
		</h2>

		{#if !editing}
			<!-- A registered day keeps its date: moving it means deleting it and adding it again -->
			<label class="label">
				<span class="label-text">Date</span>
				<input
					type="date"
					class="input"
					required
					max={todayIsoDate()}
					bind:value={draft.workDate}
				/>
			</label>
		{/if}

		<fieldset class="flex flex-wrap items-center gap-4">
			<legend class="label-text">Kind</legend>
			<label class="flex items-center gap-2">
				<input type="radio" class="radio" value="work" bind:group={draft.kind} />
				<span>Work day</span>
			</label>
			<label class="flex items-center gap-2">
				<input type="radio" class="radio" value="flex" bind:group={draft.kind} />
				<span>Flex day (taken from the flex balance)</span>
			</label>
		</fieldset>

		{#if draft.kind === 'work'}
			<label class="flex items-start gap-2">
				<input type="checkbox" class="mt-1 checkbox" bind:checked={draft.bonus} />
				<span>
					Bonus day: no work expected
					<span class="block text-sm opacity-75">
						E.g. working on a public holiday. All worked time counts as plus time.
					</span>
				</span>
			</label>

			<div class="grid grid-cols-2 gap-3">
				<label class="label">
					<span class="label-text">Start</span>
					<input type="time" class="input" required bind:value={draft.start} />
				</label>
				<label class="label">
					<span class="label-text">End</span>
					<input type="time" class="input" bind:value={draft.end} />
				</label>
			</div>
			<p class="text-sm opacity-75">
				Leave the end empty while the day is still going. An end before the start is the next day.
			</p>

			<fieldset class="space-y-3">
				<legend class="label-text">Pauses</legend>
				{#each draft.pauses as pause, index (pause)}
					{@const label = `Pause ${index + 1}`}
					<div class="space-y-2 card preset-outlined-surface-200-800 p-3">
						<div class="flex items-center gap-2">
							<select
								class="select grow capitalize"
								aria-label="{label} kind"
								bind:value={pause.kind}
							>
								{#each PAUSE_KINDS as kind (kind)}
									<option value={kind}>{PAUSE_NAMES[kind]}</option>
								{/each}
							</select>
							<button
								type="button"
								class="btn-icon preset-tonal"
								aria-label="Remove {label.toLowerCase()}"
								onclick={() => draft.pauses.splice(index, 1)}
							>
								<X class="size-4" aria-hidden="true" />
							</button>
						</div>
						<!-- Two columns: three time inputs side by side are too narrow on a phone with am/pm -->
						<div class="grid grid-cols-2 gap-2">
							<label class="label">
								<span class="label-text">Start</span>
								<input
									type="time"
									class="input"
									aria-label="{label} start"
									bind:value={pause.start}
								/>
							</label>
							<label class="label">
								<span class="label-text">End</span>
								<input type="time" class="input" aria-label="{label} end" bind:value={pause.end} />
							</label>
							<label class="label">
								<span class="label-text">Or length</span>
								<input
									class="input"
									placeholder="0:45"
									aria-label="{label} length"
									bind:value={pause.duration}
								/>
							</label>
						</div>
					</div>
				{/each}
				<button
					type="button"
					class="btn preset-tonal"
					onclick={() =>
						draft.pauses.push(emptyPause(draft.pauses.length === 0 ? 'lunch' : 'break'))}
				>
					<Plus class="size-4" aria-hidden="true" />
					<span>Add pause</span>
				</button>
				<p class="text-sm opacity-75">
					Without a logged lunch or break, the expected one from the schedule counts.
				</p>
			</fieldset>
		{/if}

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
	title="Delete this day?"
	message="Its times, pauses and note are removed, and it no longer counts in the flex balance."
	confirmLabel="Delete"
	onconfirm={remove}
/>
