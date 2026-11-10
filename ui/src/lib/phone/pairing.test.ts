import { describe, expect, it } from 'vitest';
import { parsePairingLink } from './pairing.js';

const CODE = 'K7QX-2MDA-PW4R-HNBE-3TYC-ZUA2-MD';

describe('parsePairingLink', () => {
	it('reads what the Mac encodes', () => {
		const link = `nookery://pair?code=${CODE}&addr=192.168.0.5%3A47480&name=Anna%27s%20Mac%20mini`;
		expect(parsePairingLink(link)).toEqual({
			code: CODE,
			addr: '192.168.0.5:47480',
			name: "Anna's Mac mini"
		});
	});

	it('tolerates whitespace, lower case and a missing name or port', () => {
		expect(parsePairingLink(`  nookery://pair?code=${CODE.toLowerCase()}&addr=10.0.2.2\n`)).toEqual(
			{
				code: CODE,
				addr: '10.0.2.2',
				name: ''
			}
		);
	});

	it('refuses anything else', () => {
		for (const bad of [
			'',
			CODE,
			'https://pair?code=' + CODE + '&addr=1.2.3.4',
			'nookery://other?code=' + CODE + '&addr=1.2.3.4',
			'nookery://pair?addr=1.2.3.4',
			'nookery://pair?code=NOPE&addr=1.2.3.4',
			`nookery://pair?code=${CODE}`,
			`nookery://pair?code=${CODE}&addr=evil.com/path`,
			`nookery://pair?code=${CODE}&addr=1.2.3.4:99999`,
			`nookery://pair?code=${CODE}&addr=1.2.3.4:0`
		])
			expect(parsePairingLink(bad), bad).toBeNull();
	});
});
