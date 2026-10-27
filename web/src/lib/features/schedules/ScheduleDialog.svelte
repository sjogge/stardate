<script lang="ts">
	import type { Schedule } from '$lib/api/types';
	import { formatWeekday, todayIsoDate } from '$lib/format';
	import {
		WEEKDAYS,
		copyMondayToWeekdays,
		draftFrom,
		toRequest,
		type ScheduleDraft
	} from './scheduleForm';
	import type { Schedules } from './schedules.svelte';

	let { schedules }: { schedules: Schedules } = $props();

	const id = $props.id();
	// Undefined once the component is gone, e.g. after leaving the page while a save is in flight
	let dialog: HTMLDialogElement | undefined;
	let editing = $state<Schedule>();
	let draft = $state<ScheduleDraft>(draftFrom(undefined, ''));
	let error = $state<string>();

	/** Edits `schedule`, or starts a new version as a copy of the latest one */
	export function open(schedule?: Schedule): void {
		editing = schedule;
		// A first schedule gets no date: days before it cannot be registered, so it is a
		// deliberate choice (usually the first day you track), not silently today
		draft = schedule
			? draftFrom(schedule, schedule.validFrom)
			: draftFrom(schedules.latest, schedules.latest ? todayIsoDate() : '');
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
		class="flex min-h-0 w-full flex-col gap-4 overflow-y-auto md:w-lg"
		novalidate
		onsubmit={submit}
	>
		<h2 id="{id}-title" class="h4">{editing ? 'Edit schedule' : 'New schedule'}</h2>

		<label class="label">
			<span class="label-text">Applies to days from</span>
			<input type="date" class="input" required bind:value={draft.validFrom} />
		</label>
		{#if !editing && !schedules.latest}
			<p class="text-sm">
				For a first schedule, choose the first day you track: days before it cannot be registered.
			</p>
		{/if}
		<p class="text-sm opacity-75">
			It applies until the next schedule starts. Days already registered keep the times they were
			registered with, so changing a schedule never changes your history.
		</p>

		{@render paidChoice('Lunch', draft.lunchPaid, (paid) => (draft.lunchPaid = paid))}
		{@render paidChoice('Breaks', draft.breaksPaid, (paid) => (draft.breaksPaid = paid))}

		<p id="{id}-hours" class="text-sm opacity-75">
			Hours per weekday, e.g. 8:00. Leave a day empty when no work is expected.
		</p>
		<table class="table" aria-label="Hours per weekday" aria-describedby="{id}-hours">
			<thead>
				<tr>
					<th scope="col">Day</th>
					<th scope="col">Work</th>
					<th scope="col">Lunch</th>
					<th scope="col">Breaks</th>
				</tr>
			</thead>
			<tbody>
				{#each WEEKDAYS as weekday, index (weekday)}
					{@const name = formatWeekday(weekday)}
					<tr>
						<th scope="row">{name}</th>
						<td>
							<input
								class="input"
								placeholder="0:00"
								aria-label="{name} work"
								bind:value={draft.days[index].work}
							/>
						</td>
						<td>
							<input
								class="input"
								placeholder="0:00"
								aria-label="{name} lunch"
								bind:value={draft.days[index].lunch}
							/>
						</td>
						<td>
							<input
								class="input"
								placeholder="0:00"
								aria-label="{name} breaks"
								bind:value={draft.days[index].breaks}
							/>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
		<button
			type="button"
			class="btn self-start preset-tonal"
			onclick={() => copyMondayToWeekdays(draft)}
		>
			Copy Monday to Tuesday–Friday
		</button>

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
