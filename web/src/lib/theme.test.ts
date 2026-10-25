import { afterEach, describe, expect, it, vi } from 'vitest';

// The module reads storage once when it loads, so each test imports a fresh copy
async function loadTheme(stored: Record<string, string> = {}) {
	const values = new Map(Object.entries(stored));
	vi.stubGlobal('localStorage', {
		getItem: (key: string) => values.get(key) ?? null,
		setItem: (key: string, value: string) => values.set(key, value)
	});
	vi.resetModules();
	const theme = await import('./theme.svelte');
	return { ...theme, values };
}

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('appearance', () => {
	it('uses the stored theme and mode', async () => {
		const { appearance } = await loadTheme({ 'stardate-theme': 'pine', 'stardate-mode': 'dark' });

		expect(appearance.theme).toBe('pine');
		expect(appearance.mode).toBe('dark');
	});

	it('falls back to the defaults for unknown stored values', async () => {
		const { appearance } = await loadTheme({ 'stardate-theme': 'neon', 'stardate-mode': 'dim' });

		expect(appearance.theme).toBe('cerberus');
		expect(appearance.mode).toBe('system');
	});

	it('falls back to the defaults when storage is blocked', async () => {
		vi.resetModules();
		vi.stubGlobal('localStorage', {
			getItem: () => {
				throw new DOMException('Blocked', 'SecurityError');
			}
		});
		const { appearance } = await import('./theme.svelte');

		expect(appearance.theme).toBe('cerberus');
	});

	it('persists a new choice', async () => {
		const { appearance, values } = await loadTheme();

		appearance.setTheme('rose');
		appearance.setMode('light');

		expect(values.get('stardate-theme')).toBe('rose');
		expect(values.get('stardate-mode')).toBe('light');
		expect(appearance.shownMode).toBe('light');
	});
});
