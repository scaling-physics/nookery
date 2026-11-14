import { describe, expect, it } from 'vitest';
import { piecesFor } from './ingredients.js';
import type { Piece } from './types.js';

/** The first piece one ingredient gives (alone, no fallback). */
const one = (s: string): Piece | undefined => piecesFor([s])[0];

describe('piecesFor: one ingredient, English', () => {
	const cases: [string, Piece][] = [
		['250 g spinach', 'spinach'],
		['2 eggs', 'egg'],
		['cherry tomatoes', 'tomato'],
		['1 can chickpeas', 'chickpea'],
		['200 g red lentils', 'lentil'],
		['3 large mushrooms, sliced', 'mushroom'],
		['1 red bell pepper', 'pepper'],
		['2 yellow peppers', 'pepper'],
		['1 red chili', 'chili'],
		['2 onions, diced', 'onion'],
		['4 spring onions', 'spring-onion'],
		['1 leek', 'spring-onion'],
		['3 carrots', 'carrot'],
		['1 head broccoli', 'broccoli'],
		['1 cauliflower', 'broccoli'],
		['a handful of fresh basil', 'basil'],
		['1 ripe avocado', 'avocado'],
		['half a cucumber', 'cucumber'],
		['1 cup sweetcorn', 'corn'],
		['150 g frozen peas', 'pea'],
		['black olives', 'olive'],
		['500 g potatoes', 'potato'],
		['300 g king prawns', 'prawn'],
		['2 salmon fillets', 'salmon'],
		['2 chicken breasts', 'chicken'],
		['4 rashers bacon', 'bacon'],
		['2 sausages', 'sausage'],
		['400 g ground beef', 'mince'],
		['100 g grated parmesan', 'cheese'],
		['1 ball of mozzarella', 'mozzarella'],
		['1 tbsp sesame seeds', 'sesame'],
		['pumpkin seeds', 'seed'],
		['2 bananas', 'banana'],
		['1 apple', 'apple'],
		['1 orange', 'citrus'],
		['1 zucchini', 'zucchini'],
		['1 eggplant', 'aubergine'],
		['300 g butternut squash', 'pumpkin'],
		['2 beetroot', 'beet'],
		['1/4 red cabbage', 'cabbage'],
		['200 g firm tofu', 'cube'],
		['1 lemon', 'lemon'],
		['Greek yogurt', 'dollop'],
		['2 slices bread', 'crouton'],
		['a bunch of parsley', 'herb'],
		['mixed salad leaves', 'leaf'],
		['kidney beans', 'bean'],
		['1 tsp ground cumin', 'sprinkle'],
		['1 tsp smoked paprika', 'sprinkle'],
		['2 tbsp soy sauce', 'swoosh'],
		['handful of walnuts', 'nut'],
		['strawberries', 'berry']
	];
	for (const [input, piece] of cases) {
		it(`${input} -> ${piece}`, () => expect(one(input)).toBe(piece));
	}
});

describe('piecesFor: one ingredient, German', () => {
	const cases: [string, Piece][] = [
		['250 g Spinat', 'spinach'],
		['2 Eier', 'egg'],
		['Kichererbsen (Dose)', 'chickpea'],
		['rote Paprika', 'pepper'],
		['2 Paprikaschoten', 'pepper'],
		['1 Zwiebel, gewürfelt', 'onion'],
		['Halloumi, gerieben', 'cheese'],
		['Halloumi', 'cube'],
		['Lachsfilet', 'salmon'],
		['4 Frühlingszwiebeln', 'spring-onion'],
		['250 g Champignons', 'mushroom'],
		['2 Hähnchenbrustfilets', 'chicken'],
		['400 g Hackfleisch', 'mince'],
		['gemischtes Hack', 'mince'],
		['100 g Schinkenwürfel', 'slice'],
		['Speckwürfel', 'bacon'],
		['200 g Feta', 'cheese'],
		['150 g Gouda', 'cheese'],
		['Parmesan', 'cheese'],
		['125 g Mozzarella', 'mozzarella'],
		['800 g Kartoffeln', 'potato'],
		['3 Möhren', 'carrot'],
		['3 Karotten', 'carrot'],
		['1 Zucchini', 'zucchini'],
		['1 Aubergine', 'aubergine'],
		['500 g Kürbis', 'pumpkin'],
		['2 Rote Bete', 'beet'],
		['1 Stange Lauch', 'spring-onion'],
		['200 g Bohnen', 'bean'],
		['150 g Linsen', 'lentil'],
		['1 Dose Mais', 'corn'],
		['Erbsen', 'pea'],
		['Garnelen', 'prawn'],
		['Thunfisch', 'fillet'],
		['Rinderhack', 'mince'],
		['Bratwurst', 'sausage'],
		['Salatgurke', 'cucumber'],
		['1 Avocado', 'avocado'],
		['Rotkohl', 'cabbage'],
		['Blumenkohl', 'broccoli'],
		['Rucola', 'leaf'],
		['Basilikum', 'basil'],
		['frische Petersilie', 'herb'],
		['Naturjoghurt', 'dollop'],
		['2 Scheiben Toastbrot', 'crouton'],
		['Äpfel', 'apple'],
		['1 Banane', 'banana'],
		['Zitrone', 'lemon'],
		['Walnüsse', 'nut'],
		['Kürbiskerne', 'seed'],
		['Sesam', 'sesame'],
		['grüne Oliven', 'olive'],
		['Tomaten', 'tomato'],
		['Paprikapulver', 'sprinkle'],
		['1 TL Currypulver', 'sprinkle'],
		['Sojasauce', 'swoosh']
	];
	for (const [input, piece] of cases) {
		it(`${input} -> ${piece}`, () => expect(one(input)).toBe(piece));
	}
});

describe('piecesFor: pepper, bell pepper, paprika', () => {
	it('"pepper" is the spice: nothing', () => {
		expect(piecesFor(['pepper'])).toEqual([]);
		expect(piecesFor(['Salt and black pepper'])).toEqual([]);
		expect(piecesFor(['Pfeffer'])).toEqual([]);
		expect(piecesFor(['freshly ground black pepper'])).toEqual([]);
	});
	it('bell pepper and Paprika are the vegetable', () => {
		expect(piecesFor(['1 bell pepper'])).toEqual(['pepper']);
		expect(piecesFor(['Paprika'])).toEqual(['pepper']);
		expect(piecesFor(['Peperoni'])).toEqual(['pepper']);
	});
	it('Paprikapulver and pepper flakes are a sprinkle', () => {
		expect(piecesFor(['Paprikapulver'])).toEqual(['sprinkle']);
		expect(piecesFor(['chili flakes'])).toEqual(['sprinkle']);
	});
	it('pepperoni is a sausage, peppermint a herb', () => {
		expect(piecesFor(['pepperoni'])).toEqual(['sausage']);
		expect(piecesFor(['peppermint'])).toEqual(['herb']);
	});
});

describe('piecesFor: staples and bases give nothing', () => {
	for (const s of [
		'Salz',
		'1 tsp salt',
		'2 tbsp olive oil',
		'Olivenöl',
		'Sonnenblumenöl zum Braten',
		'50 g butter',
		'200 ml water',
		'500 ml vegetable stock',
		'Gemüsebrühe',
		'chicken stock cube',
		'2 tbsp flour',
		'Mehl',
		'1 tsp sugar',
		'Zucker',
		'1 tbsp vinegar',
		'Zitronensaft',
		'juice of 1 lemon',
		'200 ml milk',
		'100 ml Sahne',
		'250 g rice',
		'Basmatireis',
		'250 g Spaghetti',
		'Nudeln',
		'400 g pasta',
		'Couscous',
		'1 tortilla',
		'Pizzateig',
		'Tomatenmark',
		'Weißwein',
		'vanilla extract',
		'1 Lorbeerblatt',
		'2 tbsp tomato paste',
		'Reis in Gemüsebrühe'
	]) {
		it(s, () => expect(piecesFor([s])).toEqual([]));
	}
});

describe('piecesFor: lists', () => {
	it('keeps input order and is distinct', () => {
		expect(piecesFor(['2 eggs', '200 g spinach', '1 egg', '100 g feta'])).toEqual([
			'egg',
			'spinach',
			'cheese'
		]);
	});
	it('stops at max (default 6)', () => {
		const all = ['egg', 'spinach', 'feta', 'carrot', 'peas', 'corn', 'bacon', 'olives'];
		expect(piecesFor(all)).toHaveLength(6);
		expect(piecesFor(all, [], 3)).toEqual(['egg', 'spinach', 'cheese']);
		expect(piecesFor(all, [], 0)).toEqual([]);
	});
	it('splits "X and Y" and ignores what follows a comma', () => {
		expect(piecesFor(['Karotten und Zwiebeln'])).toEqual(['carrot', 'onion']);
		expect(piecesFor(['Zwiebel, in Öl angebraten'])).toEqual(['onion']);
		expect(piecesFor(['Thunfisch in Öl'])).toEqual(['fillet']);
	});
	it('garlic and spices only fill room', () => {
		expect(piecesFor(['3 cloves garlic', 'tomatoes', 'basil', 'mushrooms'])).toEqual([
			'tomato',
			'basil',
			'mushroom'
		]);
		expect(piecesFor(['2 Knoblauchzehen'])).toEqual(['garlic']);
		expect(piecesFor(['Knoblauch', 'Pilze'])).toEqual(['mushroom', 'garlic']);
		expect(piecesFor(['cumin', 'carrot', 'peas', 'corn', 'egg', 'bacon', 'olives'])).toEqual([
			'carrot',
			'pea',
			'corn',
			'egg',
			'bacon',
			'olive'
		]);
	});
	it('at most one sprinkle', () => {
		expect(piecesFor(['cumin', 'turmeric', 'cinnamon'])).toEqual(['sprinkle']);
	});
	it('a full recipe', () => {
		expect(
			piecesFor([
				'200 g Spaghetti',
				'2 Eier',
				'100 g Speckwürfel',
				'50 g Parmesan, gerieben',
				'Salz',
				'schwarzer Pfeffer'
			])
		).toEqual(['egg', 'bacon', 'cheese']);
	});
});

describe('piecesFor: category fallback', () => {
	it('uses the categories for names it does not know', () => {
		expect(piecesFor(['Zaubergemüse'], ['vegetables'])).toEqual(['leaf', 'tomato', 'wedge']);
		expect(piecesFor(['xyzzy foo'], ['meat', 'cheese'])).toEqual(['slice', 'cube']);
	});
	it('uses them when nothing shows', () => {
		expect(piecesFor(['Salz', 'Öl'], ['fish'])).toEqual(['fillet']);
		expect(piecesFor([], ['dairy'])).toEqual(['dollop']);
	});
	it('known names win and categories only add to a sparse list', () => {
		expect(piecesFor(['spinach', 'mystery'], ['meat'])).toEqual(['spinach', 'slice']);
		expect(piecesFor(['egg', 'spinach', 'feta', 'mystery'], ['meat'])).toEqual([
			'egg',
			'spinach',
			'cheese'
		]);
	});
	it('no categories, no fallback', () => {
		expect(piecesFor(['mystery'])).toEqual([]);
	});
});
