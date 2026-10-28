<script lang="ts">
	import type { Schedule } from '$lib/api/types';
	import { formatWeekday } from '$lib/format';
	import {
		WEEKDAYS,
		copyMondayToWeekdays,
		draftFrom,
		duplicateDraft,
		toRequest,
		type ScheduleDraft
	} from './scheduleForm';
	import type { Schedules } from './schedules.svelte';

	// Mirrors the backend's limit (schedules.rs) so the input stops there; the backend still checks
	const MAX_NAME_LENGTH = 100;

	let { schedules }: { schedules: Schedules } = $props();

	const id = $props.id();
	// Undefined once the component is gone, e.g. after leaving the page while a save is in flight
	let dialog: HTMLDialogElement | undefined;
	let editing = $state<Schedule>();
	let draft = $state<ScheduleDraft>(draftFrom(undefined));
	let error = $state<string>();

	/** The other schedules this one can rotate into; "the same schedule again" covers itself */
	const nextOptions = $derived(schedules.schedules.filter((other) => other.id !== editing?.id));

	/** Edits `schedule`, or starts a new one: empty, or as a copy of `copyOf` */
	export function open(schedule?: Schedule, copyOf?: Schedule): void {
		editing = schedule;
		draft = schedule ? draftFrom(schedule) : copyOf ? duplicateDraft(copyOf) : draftFrom(undefined);
		error = undefined;
		dialog?.showModal();
	}

	async function submit(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		const result = toRequest(draft);
		if ('error' in result) {
			error = result.error;
			return;
		}
		error = undefined;
		if (await schedules.save(result.request, editing)) dialog?.close();
	}
</script>

{#snippet paidChoice(legend: string, value: boolean | undefined, onchange: (paid: boolean) => void)}
	<fieldset class="flex flex-wrap items-center gap-4">
		<legend class="label-text">{legend}</legend>
		{#each [true, false] as paid (paid)}
			<label class="flex items-center gap-2">
				<input
					type="radio"
					class="radio"
					name="{id}-{legend}"
					required
					checked={value === paid}
					onchange={() => onchange(paid)}
				/>
				<span>{paid ? 'Paid' : 'Unpaid'}</span>
			</label>
		{/each}
	</fieldset>
{/snippet}

<!-- Full screen on phones, a centered dialog from md; the form scrolls inside either -->
<dialog
	bind:this={dialog}
	aria-labelledby="{id}-title"
	class="dialog preset-filled-surface-100-900 animate-dialog max-md:dialog-fullscreen md:max-h-[90dvh]"
>
	<!-- novalidate: toRequest explains what is missing in one alert instead of browser bubbles -->
	<form
		class="flex min-h-0 w-full flex-col gap-4 overflow-y-auto md:w-2xl"
		novalidate
		onsubmit={submit}
	>
		<h2 id="{id}-title" class="h4">{editing ? `Edit ${editing.name}` : 'New schedule'}</h2>

		<label class="label">
			<span class="label-text">Name</span>
			<input
				class="input"
				placeholder="e.g. Daytime, Nights, Week 2"
				required
				maxlength={MAX_NAME_LENGTH}
				bind:value={draft.name}
			/>
		</label>

		{@render paidChoice('Lunch', draft.lunchPaid, (paid) => (draft.lunchPaid = paid))}
		{@render paidChoice('Breaks', draft.breaksPaid, (paid) => (draft.breaksPaid = paid))}

		<fieldset class="space-y-3">
			<legend class="label-text">Shifts</legend>
			<p class="text-sm opacity-75">
				Planned start and end per weekday; an end before the start is the next day. Leave a day
				empty when no work is expected. A shift belongs to the day it starts.
			</p>
			{#each WEEKDAYS as weekday, index (weekday)}
				{@const name = formatWeekday(weekday)}
				<div
					class="grid grid-cols-[3rem_1fr_1fr] items-end gap-2 md:grid-cols-[3rem_1fr_1fr_1fr_1fr]"
				>
					<span class="pb-2 font-bold">{name}</span>
					<label class="label">
						<span class="label-text">Start</span>
						<input
							type="time"
							class="input"
							aria-label="{name} start"
							bind:value={draft.days[index].start}
						/>
					</label>
					<label class="label">
						<span class="label-text">End</span>
						<input
							type="time"
							class="input"
							aria-label="{name} end"
							bind:value={draft.days[index].end}
						/>
					</label>
					<!-- Below start and end on phones: four inputs in a row are too narrow there -->
					<label class="label max-md:col-start-2">
						<span class="label-text">Lunch</span>
						<input
							class="input"
							placeholder="0:00"
							aria-label="{name} lunch"
							bind:value={draft.days[index].lunch}
						/>
					</label>
					<label class="label">
						<span class="label-text">Breaks</span>
						<input
							class="input"
							placeholder="0:00"
							aria-label="{name} breaks"
							bind:value={draft.days[index].breaks}
						/>
					</label>
				</div>
			{/each}
			<button
				type="button"
				class="btn self-start preset-tonal"
				onclick={() => copyMondayToWeekdays(draft)}
			>
				Copy Monday to Tuesday–Friday
			</button>
		</fieldset>

		<label class="label">
			<span class="label-text">Next week</span>
			<select
				class="select"
				value={draft.nextScheduleId ?? ''}
				onchange={(event) => {
					const { value } = event.currentTarget;
					draft.nextScheduleId = value === '' ? null : Number(value);
				}}
			>
				<option value="">The same schedule again</option>
				{#each nextOptions as option (option.id)}
					<option value={option.id}>{option.name}</option>
				{/each}
			</select>
		</label>
		<p class="text-sm opacity-75">
			For rotating shifts: the schedule suggested for the week after a week on this one. You can
			always pick another when registering a day.
		</p>

		{#if error}
			<p role="alert" class="card preset-tonal-error p-3">{error}</p>
		{/if}

		<div class="flex justify-end gap-2">
			<button type="button" class="btn preset-tonal" onclick={() => dialog?.close()}>Cancel</button>
			<button type="submit" class="btn preset-filled-primary-500" disabled={schedules.pending}>
				Save
			</button>
		</div>
	</form>
</dialog>
