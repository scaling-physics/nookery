// Records the README's GIFs and stills from the UI running on its SYNTHETIC
// browser data (never the real kitchen). Run: pnpm readme:media
//
//   README_MEDIA_URL=http://localhost:5420   reuse a running dev server
//   README_MEDIA_ONLY=kitchen,phone          only these flows
//   README_MEDIA_CHANNEL=chrome              use installed Chrome, not Playwright's Chromium
//
// Frames are screenshots with durations set by the script (not by how fast
// the machine is), identical neighbours are merged, and assemble.py writes
// the GIFs. A hash of the UI sources + this folder goes to docs/media/.source-hash
// so CI can skip the whole thing when nothing it depends on changed.
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const outDir = path.join(root, 'docs/media');
const require = createRequire(path.join(root, 'ui/package.json'));

// (playwright is loaded lazily so `--hash` works before anything is installed)
const only = (process.env.README_MEDIA_ONLY ?? '').split(',').filter(Boolean);
const wanted = (n) => only.length === 0 || only.includes(n);

// ---------- source hash (used by CI to skip unchanged runs) ----------
export function sourceHash() {
	const h = createHash('sha256');
	const walk = (d) => {
		for (const e of fs.readdirSync(d, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1))) {
			if (e.name.startsWith('.')) continue;
			const p = path.join(d, e.name);
			if (e.isDirectory()) walk(p);
			else {
				h.update(path.relative(root, p));
				h.update(fs.readFileSync(p));
			}
		}
	};
	for (const d of ['ui/src', 'ui/static', 'scripts/readme-media', 'assets']) walk(path.join(root, d));
	h.update(fs.readFileSync(path.join(root, 'ui/pnpm-lock.yaml')));
	return h.digest('hex');
}
if (process.argv.includes('--hash')) {
	console.log(sourceHash());
	process.exit(0);
}

// ---------- dev server ----------
let server;
async function up(url) {
	try {
		return (await fetch(url)).ok;
	} catch {
		return false;
	}
}
async function ensureServer() {
	if (process.env.README_MEDIA_URL) return process.env.README_MEDIA_URL;
	const port = 5473;
	const url = `http://localhost:${port}`;
	if (await up(url)) return url;
	server = spawn('pnpm', ['-C', 'ui', 'exec', 'vite', 'dev', '--port', String(port), '--strictPort'], {
		cwd: root,
		stdio: 'ignore'
	});
	for (let i = 0; i < 120; i++) {
		if (await up(url)) return url;
		await new Promise((r) => setTimeout(r, 500));
	}
	throw new Error('the dev server did not start');
}

// ---------- recorder ----------
const CURSOR = `
(() => {
  const css = document.createElement('style');
  css.textContent = \`
   #__cur{position:fixed;left:0;top:0;width:26px;height:26px;z-index:2147483647;pointer-events:none;
     transform:translate(-100px,-100px);filter:drop-shadow(0 1px 2px rgba(0,0,0,.35))}
   .__rip{position:fixed;z-index:2147483646;pointer-events:none;border-radius:50%;
     width:44px;height:44px;margin:-22px 0 0 -22px;background:rgba(74,111,58,.28);
     border:2px solid rgba(74,111,58,.7);transform:scale(.4);opacity:1}
   .__rip.go{transform:scale(1);opacity:.0;transition:transform .35s ease-out,opacity .35s ease-out}
   .__tap #__cur{display:none}
  \`;
  const add = () => {
    if (document.getElementById('__cur')) return;
    document.head.appendChild(css);
    const c = document.createElement('div');
    c.id = '__cur';
    c.innerHTML = '<svg viewBox="0 0 24 24" width="26" height="26"><path d="M4 2l15 10-6.5 1.4L9.5 20z" fill="#2a2118" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/></svg>';
    document.body.appendChild(c);
    if (window.__TAP) document.documentElement.classList.add('__tap');
    addEventListener('mousemove', (e) => { c.style.transform = 'translate(' + (e.clientX - 4) + 'px,' + (e.clientY - 2) + 'px)'; }, true);
    addEventListener('mousedown', (e) => {
      const r = document.createElement('div');
      r.className = '__rip'; r.style.left = e.clientX + 'px'; r.style.top = e.clientY + 'px';
      document.body.appendChild(r);
      requestAnimationFrame(() => requestAnimationFrame(() => r.classList.add('go')));
      setTimeout(() => r.remove(), 600);
    }, true);
  };
  if (document.body) add(); else addEventListener('DOMContentLoaded', add);
})();`;

// A fixed "today" so the dates on screen ("Week of 9 Nov", "use today") do not drift.
async function page0clock(ctx) {
	await ctx.clock.install({ time: new Date('2026-11-09T09:30:00+01:00') });
}

const ease = (t) => (t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

class Rec {
	constructor(page, name, { width, tap = false }) {
		this.page = page;
		this.name = name;
		this.width = width;
		this.tap = tap;
		this.frames = [];
		this.mouse = { x: 40, y: 40 };
	}
	/** One frame, shown for `ms`. Identical neighbours are merged. */
	async snap(ms = 100) {
		// two animation frames so a scroll has been painted before we look at it
		await this.page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
		const buf = await this.page.screenshot({ type: 'png', animations: 'allow' });
		const last = this.frames[this.frames.length - 1];
		if (last && last.buf.equals(buf)) last.ms += ms;
		else this.frames.push({ buf, ms });
	}
	/** Hold the current picture. */
	async hold(ms) {
		// (the picture is shown for `ms`; the page only needs a moment of real time to settle)
		await this.page.waitForTimeout(Math.min(ms, 450));
		await this.snap(ms);
	}
	/** Real time passes (CSS animations run); a frame every `every` ms. */
	async play(total, every = 100) {
		for (let t = 0; t < total; t += every) {
			await this.page.waitForTimeout(every - 40 > 0 ? every - 40 : 0);
			await this.snap(every);
		}
	}
	async debug() {
		const f = path.join(os.tmpdir(), `nookery-${this.name}-failed.png`);
		await this.page.screenshot({ path: f }).catch(() => {});
		console.error('screenshot of the failure:', f);
	}
	async center(sel) {
		const loc = typeof sel === 'string' ? this.page.locator(sel).first() : sel;
		await loc.scrollIntoViewIfNeeded({ timeout: 6000 }).catch(async (e) => {
			await this.debug();
			throw e;
		});
		const b = await loc.boundingBox();
		if (!b) throw new Error(`no box for ${sel}`);
		return { x: b.x + b.width / 2, y: b.y + b.height / 2, loc };
	}
	async glide(sel, { steps = 7, ms = 60 } = {}) {
		const { x, y, loc } = await this.center(sel);
		const from = this.mouse;
		for (let i = 1; i <= steps; i++) {
			const e = ease(i / steps);
			await this.page.mouse.move(from.x + (x - from.x) * e, from.y + (y - from.y) * e);
			await this.snap(ms);
		}
		this.mouse = { x, y };
		return loc;
	}
	async click(sel, { before = 150, after = 350 } = {}) {
		const loc = await this.glide(sel);
		await this.hold(before);
		await this.page.mouse.down();
		await this.page.waitForTimeout(60);
		await this.snap(90);
		await this.page.mouse.up();
		await this.page.waitForTimeout(120);
		await this.snap(after > 200 ? 120 : after);
		if (after > 200) await this.hold(after - 120);
		return loc;
	}
	async type(sel, text, { perChar = 85, every = 2 } = {}) {
		await this.click(sel, { after: 150 });
		let n = 0;
		for (const ch of text) {
			await this.page.keyboard.type(ch);
			if (++n % every === 0 || n === text.length) await this.snap(perChar * every);
		}
	}
	async scrollTo(y, { steps = 8, ms = 55 } = {}) {
		const from = await this.page.evaluate(() => window.scrollY);
		for (let i = 1; i <= steps; i++) {
			await this.page.evaluate((v) => window.scrollTo(0, v), from + (y - from) * ease(i / steps));
			await this.page.waitForTimeout(140); // let Chrome raster the new part before we look
			await this.snap(ms);
		}
	}
	async scrollIntoView(sel, offset = 90) {
		const y = await this.page.locator(sel).first().evaluate((el, o) => el.getBoundingClientRect().top + scrollY - o, offset);
		await this.scrollTo(Math.max(0, y));
	}
	async save({ colors = 96, lastHold = 1600 } = {}) {
		if (this.frames.length) this.frames[this.frames.length - 1].ms = Math.max(this.frames.at(-1).ms, lastHold);
		const dir = fs.mkdtempSync(path.join(os.tmpdir(), `nookery-${this.name}-`));
		const manifest = this.frames.map((f, i) => {
			const file = `${String(i).padStart(4, '0')}.png`;
			fs.writeFileSync(path.join(dir, file), f.buf);
			return { file, ms: f.ms };
		});
		fs.writeFileSync(path.join(dir, 'manifest.json'), JSON.stringify(manifest));
		const out = path.join(outDir, `${this.name}.gif`);
		const r = spawnSync(
			'python3',
			[path.join(here, 'assemble.py'), path.join(dir, 'manifest.json'), out, '--width', String(this.width), '--colors', String(colors)],
			{ stdio: 'inherit' }
		);
		if (r.status !== 0) throw new Error('assemble.py failed');
		if (process.env.README_MEDIA_KEEP) console.log('frames kept in', dir);
		else fs.rmSync(dir, { recursive: true, force: true });
	}
}

// ---------- flows ----------
const DESKTOP = { width: 960, height: 600 };
const PHONE = { width: 390, height: 780 };

async function open(browser, base, route, { phone = false, scheme = 'light', name }) {
	const ctx = await browser.newContext({
		viewport: phone ? PHONE : DESKTOP,
		deviceScaleFactor: 1,
		colorScheme: scheme,
		hasTouch: phone,
		isMobile: false,
		reducedMotion: 'no-preference',
		locale: 'en-GB',
		timezoneId: 'Europe/Berlin'
	});
	await page0clock(ctx);
	await ctx.addInitScript(`window.__TAP = ${phone}; try { localStorage.clear(); } catch {}`);
	await ctx.addInitScript(CURSOR);
	const page = await ctx.newPage();
	await page.goto(base + route);
	await page.waitForLoadState('networkidle');
	await page.waitForTimeout(900);
	const rec = new Rec(page, name, { width: phone ? PHONE.width : DESKTOP.width, tap: phone });
	current = rec;
	return { ctx, page, rec };
}

/** Phone: a tap (ripple, no arrow). */
async function tap(rec, sel, { after = 400 } = {}) {
	const { x, y, loc } = await rec.center(sel);
	await rec.page.mouse.move(x, y);
	await rec.hold(120);
	await rec.page.mouse.down();
	await rec.page.waitForTimeout(60);
	await rec.snap(90);
	await rec.page.mouse.up();
	await rec.page.waitForTimeout(150);
	await rec.snap(120);
	await rec.hold(after);
	return loc;
}

let current;
const flows = {
	async kitchen(browser, base) {
		const { ctx, page, rec } = await open(browser, base, '/', { name: 'kitchen' });
		await rec.hold(1400);
		// look at "Use first"
		await rec.glide('section[aria-labelledby="soon-h"] button[aria-label^="Used up"]', { steps: 6 });
		await rec.hold(900);
		// add something by typing
		await rec.type('input[aria-label="Item"]', 'Basil', { every: 2 });
		await rec.hold(500);
		await rec.click('form[aria-label="Add something"] button:has-text("Add")', { after: 900 });
		// remove one with a tap; Undo brings it back
		// (let the add form fold away first: it shrinks on blur and would shift the row)
		await page.evaluate(() => document.activeElement?.blur());
		await rec.hold(400);
		await rec.scrollTo(300);
		await rec.hold(500);
		await rec.click('section:not([aria-labelledby="soon-h"]) button[aria-label^="Used up: Spinach"]', { after: 1300 });
		await rec.click('button:has-text("Undo")', { after: 1200 });
		await rec.save({ lastHold: 1800 });
		await ctx.close();
	},

	async recipes(browser, base) {
		const { ctx, page, rec } = await open(browser, base, '/recipes', { name: 'recipes' });
		await rec.hold(1000);
		await rec.click('label.seg:has-text("Vegetarian")', { after: 500 });
		await rec.click('button:has-text("Suggest")', { after: 100 });
		await rec.scrollTo(330, { steps: 6 });
		await rec.play(1900, 120); // the cooking loader
		await page.waitForSelector('button:has-text("Cook this")', { timeout: 15000 });
		await rec.hold(800);
		await rec.scrollIntoView('button:has-text("Cook this")', 260);
		await rec.hold(1400);
		await rec.click('button:has-text("Cook this") >> nth=1', { after: 300 });
		await rec.play(1500, 150); // the recipe is written
		await page.waitForSelector('button:has-text("Back to ideas")', { timeout: 10000 });
		await rec.hold(1200);
		await rec.scrollTo(380, { steps: 10 });
		await rec.hold(1400);
		await rec.save({ lastHold: 1500 });
		await ctx.close();
	},

	async plan(browser, base) {
		const { ctx, page, rec } = await open(browser, base, '/plan', { name: 'plan' });
		await rec.hold(900);
		await rec.click('button:has-text("Plan my week")', { after: 200 });
		await rec.play(1500, 150);
		await page.waitForSelector('ul[aria-label="Dinners this week"] li', { timeout: 15000 });
		await rec.hold(1300);
		await rec.scrollTo(280, { steps: 8 });
		await rec.hold(1200);
		await rec.scrollTo(620, { steps: 8 });
		await rec.hold(900);
		await rec.click('button:has-text("Add what")', { after: 1500 });
		await rec.save({ lastHold: 1800 });
		await ctx.close();
	},

	async shopping(browser, base) {
		// Off camera: plan a week and send it to the list (state lives in the page).
		const { ctx, page, rec } = await open(browser, base, '/plan', { name: 'shopping' });
		await page.click('button:has-text("Plan my week")');
		await page.waitForSelector('ul[aria-label="Dinners this week"] li', { timeout: 15000 });
		await page.click('button:has-text("Add what")');
		await page.waitForSelector('a:has-text("Open the shopping list")');
		await page.click('a:has-text("Open the shopping list")');
		await page.waitForSelector('input[type="checkbox"]');
		await page.mouse.move(40, 40);
		await page.waitForTimeout(600);
		await rec.hold(1200);
		for (let i = 0; i < 3; i++) await rec.click(`input[type="checkbox"] >> nth=${i}`, { after: 450 });
		await rec.hold(700);
		await rec.click('button:has-text("Put away")', { after: 600 });
		await rec.play(900, 150);
		await rec.hold(1200);
		await rec.save({ lastHold: 1800 });
		await ctx.close();
	},

	async photo(browser, base) {
		const { ctx, page, rec } = await open(browser, base, '/photo', { name: 'photo' });
		await rec.hold(900);
		await rec.click('label.seg:has-text("Fridge")', { after: 400 });
		// a plain picture stands in for the fridge photo (the scan is synthetic here)
		const dataUrl = await page.evaluate(() => {
			const c = document.createElement('canvas');
			c.width = 360;
			c.height = 480;
			const g = c.getContext('2d');
			g.fillStyle = '#f4efe6';
			g.fillRect(0, 0, 360, 480);
			const tints = ['#7da05a', '#d9a441', '#c8553d', '#e8e1d0', '#5b8fb0', '#a67c52'];
			for (let shelf = 0; shelf < 4; shelf++) {
				const y = 40 + shelf * 112;
				for (let i = 0; i < 6; i++) {
					g.fillStyle = tints[(shelf * 2 + i) % tints.length];
					const h = 40 + ((i * 37 + shelf * 23) % 40);
					g.fillRect(18 + i * 55, y + 80 - h, 42, h);
				}
				g.fillStyle = '#cfc6b3';
				g.fillRect(8, y + 80, 344, 6);
			}
			return c.toDataURL('image/png');
		});
		const png = Buffer.from(dataUrl.split(',')[1], 'base64');
		await rec.glide('label:has-text("Choose a photo")');
		await rec.hold(300);
		await page.setInputFiles('input[type="file"]', { name: 'my-fridge.png', mimeType: 'image/png', buffer: png });
		await rec.hold(700);
		await rec.click('button:has-text("Scan")', { after: 200 });
		await rec.play(2200, 150);
		await page.waitForSelector('#here-h', { timeout: 20000 });
		await rec.hold(1400);
		await rec.scrollIntoView('#new-h', 70);
		await rec.hold(1500);
		await rec.scrollIntoView('#gone-h', 70);
		await rec.hold(1800);
		await rec.save({ lastHold: 1500 });
		await ctx.close();
	},

	async phone(browser, base) {
		const { ctx, page, rec } = await open(browser, base, '/', { phone: true, name: 'phone' });
		await rec.hold(1500);
		await rec.scrollTo(260, { steps: 8 });
		await rec.hold(900);
		await rec.scrollTo(0, { steps: 6 });
		await tap(rec, 'nav[aria-label="Sections"]:visible >> text=Recipes', { after: 800 });
		await tap(rec, 'label.seg:has-text("Quick weeknight")', { after: 300 });
		await tap(rec, 'button:has-text("Suggest")', { after: 100 });
		await rec.scrollTo(520, { steps: 6 });
		await rec.play(1600, 150);
		await page.waitForSelector('button:has-text("Cook this")', { timeout: 15000 });
		await rec.hold(500);
		await rec.scrollIntoView('button:has-text("Cook this")', 360);
		await rec.hold(1500);
		await tap(rec, 'nav[aria-label="Sections"]:visible >> text=Plan', { after: 900 });
		await rec.save({ lastHold: 1500 });
		await ctx.close();
	},

	async sync(browser, base) {
		const { ctx, rec } = await open(browser, base, '/settings', { name: 'sync' });
		await rec.hold(800);
		await rec.scrollIntoView('#sync-h', 60);
		await rec.hold(700);
		await rec.click('label:has-text("Share with my phone")', { after: 400 });
		await rec.hold(900);
		await rec.scrollTo((await rec.page.evaluate(() => scrollY)) + 340, { steps: 8 });
		await rec.hold(2200);
		await rec.save({ lastHold: 1800 });
		await ctx.close();
		const p = await open(browser, base, '/settings?phone', { phone: true, name: 'sync-phone' });
		await p.rec.scrollIntoView('#sync-h', 60);
		await p.rec.hold(900);
		await p.page.fill('input[placeholder^="ABCD"]', 'RTWK-3JXM-9DQV-HN4P-CZ7B-F2LS');
		await p.rec.hold(500);
		await p.page.fill('input[placeholder^="192"]', '192.168.1.20');
		await p.rec.hold(900);
		await tap(p.rec, 'button:has-text("Pair")', { after: 400 });
		await p.rec.hold(1800);
		await p.rec.save({ lastHold: 1500 });
		await p.ctx.close();
	},

	async stills(browser, base) {
		for (const scheme of ['light', 'dark']) {
			const { ctx, page } = await open(browser, base, '/', { scheme, name: 'still' });
			await page.screenshot({ path: path.join(outDir, `kitchen-${scheme}.png`) });
			await ctx.close();
		}
		const { ctx, page } = await open(browser, base, '/settings', { name: 'still' });
		await page.screenshot({ path: path.join(outDir, 'settings.png') });
		await ctx.close();
	}
};

// ---------- main ----------
fs.mkdirSync(outDir, { recursive: true });
const { chromium } = require('playwright');
const base = await ensureServer();
let browser;
try {
	browser = await chromium.launch(process.env.README_MEDIA_CHANNEL ? { channel: process.env.README_MEDIA_CHANNEL } : {});
} catch (e) {
	console.warn('Playwright Chromium missing, trying installed Chrome…');
	browser = await chromium.launch({ channel: 'chrome' });
}
try {
	for (const [name, fn] of Object.entries(flows)) {
		if (!wanted(name)) continue;
		console.log(`# ${name}`);
		// one retry: a hot-reload of the dev server mid-flow resets the page
		for (let attempt = 1; ; attempt++) {
			try {
				await fn(browser, base);
				break;
			} catch (e) {
				if (attempt >= 2) throw e;
				console.warn(`${name} failed (${String(e.message).split('\n')[0]}), trying once more`);
			}
		}
	}
	if (only.length === 0) fs.writeFileSync(path.join(outDir, '.source-hash'), sourceHash() + '\n');
} finally {
	await browser.close();
	server?.kill();
}
