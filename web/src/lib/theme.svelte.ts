import { MediaQuery } from 'svelte/reactivity';

// Keep in sync with the pre-paint script in app.html and the theme imports in routes/layout.css;
// theme.test.ts fails when they differ
export const THEMES = [
	'catppuccin',
	'dracula',
	'mint',
	'nouveau',
	'vox',
	'wintry',
	'crimson',
	'legacy'
] as const;
export const MODES = ['light', 'dark', 'system'] as const;

export type ThemeName = (typeof THEMES)[number];
export type Mode = (typeof MODES)[number];

const THEME_KEY = 'stardate-theme';
const MODE_KEY = 'stardate-mode';
export const DEFAULT_THEME: ThemeName = 'catppuccin';
const DEFAULT_MODE: Mode = 'system';

const prefersDark = new MediaQuery('(prefers-color-scheme: dark)');

class Appearance {
	theme = $state(readStored(THEME_KEY, THEMES) ?? DEFAULT_THEME);
	mode = $state(readStored(MODE_KEY, MODES) ?? DEFAULT_MODE);

	/** What is shown: `system` follows the OS setting, also when it changes while the app is open */
	get shownMode(): Exclude<Mode, 'system'> {
		if (this.mode !== 'system') return this.mode;
		return prefersDark.current ? 'dark' : 'light';
	}

	setTheme(theme: ThemeName): void {
		this.theme = theme;
		store(THEME_KEY, theme);
	}

	setMode(mode: Mode): void {
		this.mode = mode;
		store(MODE_KEY, mode);
	}
}

export const appearance = new Appearance();

/** Applies the choice to `<html>`, where Skeleton reads `data-theme` and the dark variant `data-mode`. */
export function applyAppearance(): void {
	const root = document.documentElement;
	root.dataset.theme = appearance.theme;
	root.dataset.mode = appearance.shownMode;
}

// Storage can be unavailable (private mode, blocked site data); the defaults apply then
function readStored<T extends string>(key: string, allowed: readonly T[]): T | undefined {
	try {
		const stored = localStorage.getItem(key);
		return allowed.find((value) => value === stored);
	} catch {
		return undefined;
	}
}

function store(key: string, value: string): void {
	try {
		localStorage.setItem(key, value);
	} catch {
		// Not persisted: the choice still applies until the page is reloaded
	}
}
