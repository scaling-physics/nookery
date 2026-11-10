// Messages for the phone's AI errors. `ErrorBanner` adds a Settings link to these.
export const PHONE_MODEL_MISSING =
	'The recipe model isn’t on this phone yet. Download it from your Mac in Settings, or choose “On my Mac”.';
export const MAC_UNREACHABLE_DEFAULT =
	'Couldn’t reach your Mac. Open Nookery there, make sure sharing is on and you’re on the home Wi-Fi. You can also download the recipe model to this phone in Settings.';

/** Messages that should offer a link to Settings. */
export function needsSettings(message: string): boolean {
	return (
		message === PHONE_MODEL_MISSING ||
		message === MAC_UNREACHABLE_DEFAULT ||
		message.startsWith('No Mac is paired') ||
		message.startsWith('Couldn’t reach your Mac') ||
		message.startsWith("Couldn't reach your Mac")
	);
}
