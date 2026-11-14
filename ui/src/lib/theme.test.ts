import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { parseTheme } from './theme.svelte.js';

const css = readFileSync(new URL('../routes/layout.css', import.meta.url), 'utf8');

function decls(block: string): string[] {
	return block
		.split('\n')
		.map((l) => l.trim())
		.filter((l) => l.startsWith('--') || l.startsWith('color-scheme'));
}

describe('theme', () => {
	it('reads only light and dark from storage, everything else is auto', () => {
		expect(parseTheme('light')).toBe('light');
		expect(parseTheme('dark')).toBe('dark');
		expect(parseTheme(null)).toBe('auto');
		expect(parseTheme('sepia')).toBe('auto');
	});

	it('pins the same dark tokens the system dark mode uses', () => {
		const system = [
			...css.matchAll(
				/@media \(prefers-color-scheme: dark\) \{\s*:root:not\(\[data-theme='light'\]\) \{([^}]*)\}/g
			)
		].map((m) => decls(m[1]));
		const pinned = [...css.matchAll(/:root\[data-theme='dark'\] \{([^}]*)\}/g)].map((m) =>
			decls(m[1])
		);
		expect(system.length).toBeGreaterThan(0);
		expect(css).not.toMatch(/prefers-color-scheme: dark\) \{\s*:root \{/);
		expect(pinned).toEqual(system);
	});
});
