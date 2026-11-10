import { api } from '#lib/api.js';
import type { PhoneAiView } from '#lib/bindings.js';

/** Where recipes are written on the phone, for page copy ("Written on this phone"). */
export function ranOnPhone(v: PhoneAiView): 'on this phone' | 'on your Mac' {
	if (v.pref === 'phone') return 'on this phone';
	if (v.pref === 'mac') return 'on your Mac';
	return v.model.downloaded ? 'on this phone' : 'on your Mac';
}

export async function loadRanOn(): Promise<'on this phone' | 'on your Mac'> {
	const r = await api.getPhoneAi();
	return r.status === 'ok' ? ranOnPhone(r.data) : 'on your Mac';
}
