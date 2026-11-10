import { isAndroid } from '#lib/platform.js';

/** The phone's behaviour: the Android app, or `?phone` in a browser (synthetic
 *  data, for screenshots and development). */
export const onPhone =
	isAndroid ||
	(typeof location !== 'undefined' && new URLSearchParams(location.search).has('phone'));
