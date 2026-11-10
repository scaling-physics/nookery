// Pure helpers for the Plan and Shopping screens (and the browser fakes).

import type {
	DayView,
	FoodCategoryView,
	PlanView,
	ShoppingGroupView,
	ShoppingItemView,
	SuggestionView
} from '../bindings.js';

/** Aisle-like order of the shopping list (the backend sorts the same way). */
export const AISLES: FoodCategoryView[] = [
	'vegetables',
	'fruit',
	'herbs',
	'bread',
	'dairy',
	'cheese',
	'eggs',
	'meat',
	'fish',
	'grains',
	'legumes',
	'sauces',
	'spices',
	'nuts',
	'sweets',
	'drinks',
	'leftovers',
	'other'
];

const AISLE_NAMES: Record<FoodCategoryView, string> = {
	vegetables: 'Vegetables',
	fruit: 'Fruit',
	herbs: 'Fresh herbs',
	bread: 'Bread',
	dairy: 'Dairy',
	cheese: 'Cheese',
	eggs: 'Eggs',
	meat: 'Meat',
	fish: 'Fish',
	grains: 'Pasta, rice and grains',
	legumes: 'Beans and lentils',
	sauces: 'Sauces and oils',
	spices: 'Spices',
	nuts: 'Nuts and seeds',
	sweets: 'Sweets',
	drinks: 'Drinks',
	leftovers: 'Cooked food',
	other: 'Other'
};

export function aisleName(c: FoodCategoryView): string {
	return AISLE_NAMES[c];
}

/** Items in aisle order (then ticked last, then by name), grouped by category. */
export function groupShopping(items: ShoppingItemView[]): ShoppingGroupView[] {
	const rank = (c: FoodCategoryView) => AISLES.indexOf(c);
	const sorted = [...items].sort(
		(a, b) =>
			rank(a.category) - rank(b.category) ||
			Number(a.done) - Number(b.done) ||
			a.item.toLowerCase().localeCompare(b.item.toLowerCase())
	);
	const groups: ShoppingGroupView[] = [];
	for (const it of sorted) {
		const last = groups[groups.length - 1];
		if (last && last.category === it.category) last.items.push(it);
		else groups.push({ category: it.category, items: [it] });
	}
	return groups;
}

/** Parse "YYYY-MM-DD" as a local date (no time zone shifts). */
export function parseDate(s: string): Date {
	const [y, m, d] = s.split('-').map(Number);
	return new Date(y, m - 1, d);
}

export function isoDate(d: Date): string {
	const p = (n: number) => String(n).padStart(2, '0');
	return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

export function addDays(s: string, n: number): string {
	const d = parseDate(s);
	d.setDate(d.getDate() + n);
	return isoDate(d);
}

const WEEKDAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

export function weekdayName(date: string): string {
	return WEEKDAYS[parseDate(date).getDay()];
}

/** "9 Nov". */
export function shortDate(date: string): string {
	const d = parseDate(date);
	return `${d.getDate()} ${MONTHS[d.getMonth()]}`;
}

/** "Today", "Tomorrow", or the weekday; `today` is a "YYYY-MM-DD". */
export function dayHeading(date: string, today: string): string {
	if (date === today) return 'Today';
	if (date === addDays(today, 1)) return 'Tomorrow';
	return weekdayName(date);
}

/** The next Monday, or `today` if it is one. */
export function nextMonday(today: string): string {
	const dow = parseDate(today).getDay(); // 0 Sunday
	const back = (dow + 6) % 7; // days since Monday
	return back === 0 ? today : addDays(today, 7 - back);
}

/** "Week of 9 Nov". */
export function weekLabel(week: string): string {
	return `Week of ${shortDate(week)}`;
}

/** Everything the plan still needs to buy, once each (case-insensitive),
 *  ignoring cooked days. A preview of what "Add to the shopping list" does;
 *  the backend also drops what is in stock or already listed. */
export function planNeeds(plan: PlanView): string[] {
	const seen = new Set<string>();
	const out: string[] = [];
	for (const d of plan.days) {
		if (d.cooked) continue;
		for (const n of d.need) {
			const k = n.trim().toLowerCase();
			if (k && !seen.has(k)) {
				seen.add(k);
				out.push(n.trim());
			}
		}
	}
	return out;
}

/** A plan day as the idea the "I cooked it" sheet works with. */
export function daySuggestion(d: DayView, index = 0): SuggestionView {
	return {
		index,
		name: d.name,
		why: d.why,
		minutes: d.minutes,
		have: d.have,
		need: d.need,
		needCategories: d.needCategories,
		expired: [],
		usesSoon: d.usesSoon,
		score: 0
	};
}

/** One-line summary of what a day uses, for compact cards. */
export function usesLine(d: DayView): string {
	return d.have.map((h) => h.ingredient).join(', ');
}

/** How many of the plan's dinners are cooked. */
export function cookedCount(plan: PlanView): number {
	return plan.days.filter((d) => d.cooked).length;
}
