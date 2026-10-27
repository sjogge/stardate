<script lang="ts">
	let {
		title,
		message,
		confirmLabel,
		onconfirm
	}: { title: string; message: string; confirmLabel: string; onconfirm: () => void } = $props();

	const id = $props.id();
	let dialog: HTMLDialogElement;

	export function open(): void {
		dialog.showModal();
	}

	function confirm(): void {
		dialog.close();
		onconfirm();
	}
</script>

<!-- Native <dialog>: focus moves in, Escape closes it, and the page behind is inert -->
<dialog
	bind:this={dialog}
	role="alertdialog"
	aria-labelledby="{id}-title"
	aria-describedby="{id}-message"
	class="dialog space-y-4 preset-filled-surface-100-900 animate-dialog"
>
	<h2 id="{id}-title" class="h4">{title}</h2>
	<p id="{id}-message">{message}</p>
	<div class="flex justify-end gap-2">
		<button type="button" class="btn preset-tonal" onclick={() => dialog.close()}>Cancel</button>
		<button type="button" class="btn preset-filled-error-500" onclick={confirm}>
			{confirmLabel}
		</button>
	</div>
</dialog>
