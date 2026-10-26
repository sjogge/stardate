import { readFileSync } from 'node:fs';
import { afterEach, describe, expect, it, vi } from 'vitest';
// Loaded once while the file is collected, so the first test does not pay for compiling it
// (on a cold cache that can wait for Vite's dependency bundling and exceed the test timeout)
import { DEFAULT_THEME, THEMES } from './theme.svelte';

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
		const { appearance } = await loadTheme({
			'stardate-theme': 'dracula',
			'stardate-mode': 'dark'
		});

		expect(appearance.theme).toBe('dracula');
		expect(appearance.mode).toBe('dark');
	});

	it('falls back to the defaults for unknown stored values', async () => {
		const { appearance } = await loadTheme({ 'stardate-theme': 'neon', 'stardate-mode': 'dim' });

		expect(appearance.theme).toBe('catppuccin');
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

		expect(appearance.theme).toBe('catppuccin');
	});

	it('persists a new choice', async () => {
		const { appearance, values } = await loadTheme();

		appearance.setTheme('vox');
		appearance.setMode('light');

		expect(values.get('stardate-theme')).toBe('vox');
		expect(values.get('stardate-mode')).toBe('light');
		expect(appearance.shownMode).toBe('light');
	});
});

describe('theme list', () => {
	// The list is written three times: app.html cannot import TypeScript, and CSS imports are static
	const read = (path: string) => readFileSync(new URL(path, import.meta.url), 'utf8');
	const quoted = (text: string) => [...text.matchAll(/'([a-z]+)'/g)].map(([, name]) => name);

	it('imports exactly these themes in layout.css, in menu order', () => {
		const css = read('../routes/layout.css');
		const imported = [...css.matchAll(/@skeletonlabs\/skeleton\/themes\/([a-z]+)/g)].map(
			([, name]) => name
		);

		expect(imported).toEqual([...THEMES]);
	});

	it('lists the same themes and default in the pre-paint script of app.html', () => {
		const html = read('../app.html');
		const list = /const themes = \[([^\]]*)\]/.exec(html)?.[1] ?? '';

		expect(quoted(list)).toEqual([...THEMES]);
		expect(/data-theme="([a-z]+)"/.exec(html)?.[1]).toBe(DEFAULT_THEME);
	});
});
