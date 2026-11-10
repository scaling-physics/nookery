// Plan tab state that outlives the page, so switching tabs does not throw
// away a plan that took a while to write.

import type { PlanView } from '../bindings.js';

export const ps = $state<{
	templateId: number | null;
	note: string;
	days: number;
	plan: PlanView | null;
	/** Which model wrote the plan (only known for a plan made this session). */
	model: string;
	usedCloud: boolean;
	ms: number;
}>({ templateId: null, note: '', days: 5, plan: null, model: '', usedCloud: false, ms: 0 });
