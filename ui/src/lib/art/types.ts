// Shared vocabulary of the dish library (2026-11-10). FIXED: the catalogue
// (dishes.ts), the drawing (DishArt.svelte + parts/) and the ingredient
// mapping (ingredients.ts) are written against these names. Add a name only
// together with its drawing.
//
// Canvas: 100 x 100 SVG units, table at y ~ 80, light from the top left.
// Every colour is a `Colour` name = CSS token `--c-<name>` (layout.css), so
// it follows light and dark.

/** What the food sits in or on. */
export type Vessel =
	// the original twelve
	| 'plate' // flat plate (pasta nest, fillet with sides)
	| 'salad' // wide shallow salad plate
	| 'bowl' // rice / grain / poke bowl on a foot
	| 'deep' // deep soup / curry / stew bowl
	| 'small' // small bowl: porridge, dips, sides
	| 'glass' // tall glass: smoothie, parfait, lassi
	| 'pan' // cast-iron skillet with handle
	| 'wok' // wok with handle
	| 'board' // wooden board: toast, sandwiches, bruschetta
	| 'wrap' // rolled wrap / burrito / gyros on paper
	| 'round' // round pizza / flatbread on a peel or plate
	| 'slice' // a cut slice of cake / pie / tart on a small plate
	// new
	| 'pot' // stew pot / dutch oven with lid set aside
	| 'tray' // rectangular baking dish: lasagne, gratin, traybake, casserole
	| 'skewer' // skewers on a plate: kebab, satay, souvlaki
	| 'bun' // burger / stacked sandwich seen from the side
	| 'taco' // two or three folded tacos in a stand
	| 'stack' // pancake / waffle stack on a plate
	| 'mug' // soup or hot drink mug
	| 'cup' // muffin / cupcake in a paper case
	| 'steamer' // bamboo steamer: dumplings, bao
	| 'sushi' // slate board with rolls / nigiri and a sauce dish
	| 'jar' // glass jar: overnight oats, chia, pickles
	| 'loaf'; // loaf on a board: bread, banana bread, meatloaf

/** What fills the vessel: the main body of the dish. */
export type Base =
	| 'none'
	| 'broth' // clear-ish soup surface
	| 'thick' // stew / curry / chili / dal: thick sauce surface
	| 'creamy' // cream soup / risotto / korma
	| 'rice' // a mound of rice
	| 'rice-side' // sauce on one side, rice on the other (curry plate)
	| 'fried-rice' // rice with flecks
	| 'noodles' // wok noodles tangle
	| 'ramen' // noodles in broth with toppings arranged in sectors
	| 'spaghetti' // twirled nest
	| 'short-pasta' // penne / fusilli pieces
	| 'lasagne' // layered sheets, cut corner
	| 'gnocchi'
	| 'greens' // salad leaves
	| 'grain' // couscous / quinoa / bulgur
	| 'mash'
	| 'potatoes' // roast / boiled potato pieces
	| 'fries'
	| 'omelette' // folded or flat (frittata) egg
	| 'scramble'
	| 'fried-egg' // sunny side up eggs on the base sauce (shakshuka uses this on 'thick')
	| 'pancake'
	| 'toast'
	| 'pizza' // crust + sauce disc
	| 'flatbread'
	| 'tortilla'
	| 'dumpling'
	| 'porridge'
	| 'yoghurt'
	| 'cake'
	| 'crumble'
	| 'pie' // pie / quiche / tart with crust
	| 'fillet' // a piece of fish or meat
	| 'patty' // burger patty / fritters / falafel
	| 'roast' // roast chicken / joint
	| 'traybake' // pieces spread in a tray
	| 'gratin' // browned top
	| 'sushi-roll'
	| 'smoothie';

/** Food tints: CSS `--c-<name>`. */
export type Colour =
	| 'wheat'
	| 'terra'
	| 'pumpkin'
	| 'brown'
	| 'leaf'
	| 'milk'
	| 'rust'
	| 'lemon'
	| 'plum'
	| 'pea'
	| 'blue'
	| 'muted'
	| 'cream'
	| 'dark'
	| 'pink'
	| 'olive';

/** Small ingredient glyphs placed on the base (centred at 0,0, ~ +-5 units). */
export type Piece =
	// the original toppings
	| 'leaf'
	| 'wedge'
	| 'tomato'
	| 'herb'
	| 'lemon'
	| 'cube' // cheese / tofu / feta / halloumi cube
	| 'dollop' // yoghurt / sour cream / creme fraiche
	| 'egg' // halved boiled egg
	| 'slice' // meat slice
	| 'fillet' // fish piece
	| 'grain'
	| 'bean'
	| 'swoosh' // sauce drizzle
	| 'sprinkle' // spice
	| 'nut'
	| 'berry'
	| 'crouton'
	| 'dot'
	// new
	| 'fried-egg'
	| 'chickpea'
	| 'lentil'
	| 'mushroom'
	| 'pepper' // bell pepper strip
	| 'chili'
	| 'onion' // ring
	| 'spring-onion'
	| 'garlic'
	| 'carrot' // coin
	| 'broccoli'
	| 'spinach' // wilted leaf
	| 'basil'
	| 'avocado' // slice
	| 'cucumber' // coin
	| 'corn'
	| 'pea'
	| 'olive'
	| 'potato' // piece
	| 'prawn'
	| 'salmon'
	| 'chicken' // piece
	| 'bacon'
	| 'sausage' // coin
	| 'mince' // crumbles
	| 'cheese' // grated / shavings
	| 'mozzarella'
	| 'sesame'
	| 'seed'
	| 'banana'
	| 'apple'
	| 'citrus' // orange slice
	| 'zucchini'
	| 'aubergine'
	| 'cabbage'
	| 'pumpkin' // squash cube
	| 'beet';

export interface DishKind {
	/** kebab-case, stable: stored in caches and sent to the model. */
	id: string;
	/** English name shown to the model ("Shakshuka"). */
	label: string;
	/** Lower-case match words / phrases, English and German ("shakshuka", "pfannkuchen"). */
	words: string[];
	vessel: Vessel;
	base: Base;
	/** Tint of the base (sauce, broth, dough). */
	tint: Colour;
	/** A second base on the side (curry plate: 'rice'); drawn smaller. */
	side?: Base;
	sideTint?: Colour;
	/** Always drawn, before the recipe's own ingredients (shakshuka: fried-egg). */
	signature: Piece[];
	/** Steams. */
	hot: boolean;
}

/** What the model (Settings: "the recipe model picks the picture") chose. */
export interface ArtPick {
	/** A `DishKind.id` from the catalogue. */
	kind: string;
	/** Ingredient names from the recipe worth showing, most visible first. */
	show: string[];
}
