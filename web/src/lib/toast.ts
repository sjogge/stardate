import { createToaster } from '@skeletonlabs/skeleton-svelte';

/** Rendered once in the root layout. At the top, so it never covers the bottom navigation. */
export const toaster = createToaster({ placement: 'top' });

export function showError(message: string): void {
	// The same id updates a visible toast instead of stacking identical ones
	toaster.error({ id: `error:${message}`, title: message });
}
