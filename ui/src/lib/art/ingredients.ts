// Ingredient names -> pieces to draw (English and German). Pure.
//
// Each ingredient string is cut into parts ("Salz und Pfeffer", "Zwiebel,
// gewürfelt", "Thunfisch in Öl" -> the first words only), then the FIRST
// rule that matches a part decides: a piece, or `null` = recognised but
// nothing to draw (staples, liquids, bases such as rice and pasta). Rule
// order matters: specific before generic ("paprikapulver" before
// "paprika", "frischkäse" before "käse", "olivenöl" before "olive").
// "Weak" pieces (garlic, spices, sauces) only fill room the real
// ingredients left. Names no rule knows fall back to the food categories.
import type { FoodCategoryView } from '../bindings.js';
import type { Piece } from './types.js';

const L = 'a-zäöüßéèàçñ';
/** Match at the start of a word (suffixes and plurals are free). */
const S = (src: string) => new RegExp(`(?<![${L}])(?:${src.replaceAll('@E', END)})`);
/** Match anywhere in a word (German compounds: "Champignons" ~ "pilz"). */
const A = (src: string) => new RegExp(`(?:${src.replaceAll('@E', END)})`);
const END = `(?![${L}])`;

interface Rule {
	re: RegExp;
	p: Piece | null;
	weak?: boolean;
}
const R = (re: RegExp, p: Piece | null, weak = false): Rule => ({ re, p, weak });

const RULES: Rule[] = [
	// pickles (before "essig"), creamy dairy (before "cream"/"sahne" = liquid)
	R(A('gurke|cornichon|gherkin'), 'cucumber'),
	R(A('joghurt|yoghurt|yogurt|quark'), 'dollop'),
	R(
		S(
			'frischkäse|cream cheese|quark|skyr|joghurt|yoghurt|yogurt|schmand|sour cream|crème fraîche|creme fraiche|crème fraiche|saure sahne|ricotta|hüttenkäse|cottage cheese|mascarpone|labneh|tzatziki|raita|kefir|doppelrahm|rahm@E|schlagobers|hummus|humus'
		),
		'dollop'
	),
	// --- nothing to draw: liquids, staples, baking, bases ---
	R(S('vanill'), null),
	R(A(`öl@E|oil@E|schmalz|margarine|ghee@E`), null),
	R(S(`butter@E|butterschmalz`), null),
	R(A(`salz@E|salt@E|saltwater`), null),
	R(A(`wasser@E|water@E|eiswürfel|ice cubes?@E`), null),
	R(A(`brühe|bouillon|broth@E|stock@E|fond@E|stockcube`), null),
	R(A(`saft@E|juice@E|zest@E|schale@E|abrieb|peel@E|abgerieben`), null),
	R(
		A(
			`mehl@E|flour@E|stärke|starch|cornflour|zucker@E|sugar@E|essig|vinegar|balsamic|balsamico|backpulver|natron|baking (powder|soda)|hefe@E|yeast|gelatine`
		),
		null
	),
	R(
		S(
			'(butter|hafer|mandel|soja|kokos|reis)?(milch|milk)|sahne|cream@E|schlagsahne|kondensmilch|buttermilk'
		),
		null
	),
	R(
		S(
			'(rot|weiß|weiss|koch|reis)?wein@E|wine@E|bier@E|beer@E|rum@E|brandy|likör|sherry|sake@E|mirin|whisk'
		),
		null
	),
	R(S('bay leaf|bay leaves|lorbeer|wacholder|juniper'), null),
	R(
		S(
			'tomatenmark|tomato (paste|purée|puree|sauce)|passata|passierte|gehackte tomaten|chopped tomato|crushed tomato|stückige|dosentomaten|canned tomato|tomatensauce|tomatensoße|tomatenpassata|pizzasauce'
		),
		null
	),
	R(S('paprikamark|paprikapaste|ajvar'), null),
	// bases: carbs and doughs
	R(A('(?<!p)reis(?!el)|nudel|teig|knödel'), null),
	R(
		S(
			'rice|noodle|pasta|spaghetti|penne|fusilli|tagliatelle|linguine|lasagne|makkaroni|macaroni|couscous|bulgur|quinoa|hirse|millet|polenta|grieß|semolina|haferflocken|oats?@E|rolled oats|porridge|gnocchi|ravioli|tortellini|udon|ramen|vermicelli|orzo|spätzle|maultaschen|dumpling|tortilla|wrap|dough|blätterteig|puff pastry|filo|yufka|reispapier|müsli|muesli|granola|cornflakes|cereal'
		),
		null
	),
	// --- special spices and sauces that look like vegetables ---
	R(
		S(
			'paprikapulver|paprika powder|paprikagewürz|smoked paprika|geräucherte[rn]? paprika|edelsüß|rosenpaprika|paprika edel'
		),
		'sprinkle',
		true
	),
	R(
		S(
			'(chili|chilli|pepper|pfeffer) ?(flakes|flocken|pulver|powder)|chiliflocken|cayenne|peperoncini flocken'
		),
		'sprinkle',
		true
	),
	R(S('knoblauchpulver|garlic (powder|salt)|zwiebelpulver|onion powder'), 'sprinkle', true),
	R(
		S(
			'(erdnuss|peanut|nuss|nut|mandel|almond|cashew)[- ]?(butter|mus|paste)|tahin[ie]?|sesampaste'
		),
		'swoosh',
		true
	),
	R(
		S(
			'chilisauce|chilli sauce|chili sauce|sweet chili|sriracha|sambal|harissa|tabasco|ketchup|senf@E|mustard|mayo|remoulade'
		),
		'swoosh',
		true
	),
	R(
		S(
			'soja ?s(auce|oße|osse)|soy sauce|sojasoße|tamari|worcester|fischsauce|fish sauce|austernsauce|oyster sauce|hoisin|teriyaki|barbecue|bbq|miso@E'
		),
		'swoosh',
		true
	),
	R(
		S(
			'pesto|dressing|vinaigrette|marinade|glaze|sauce@E|soße|sosse|gravy|bratensauce|sirup|syrup|honig|honey|ahornsirup|agave|karamell|caramel'
		),
		'swoosh',
		true
	),
	R(S('pepperoni'), 'sausage'),
	R(
		S(
			'(kokos|coconut) ?(raspel|flocken|flakes)|desiccated coconut|paniermehl|semmelbrösel|bread ?crumbs?|panko|(kakao|cocoa)'
		),
		'sprinkle',
		true
	),
	// --- vegetables ---
	R(
		S(
			`frühlingszwiebel|spring onion|scallion|lauchzwiebel|green onion|lauch@E|leek|porree|sellerie|celery|fenchel|fennel|spargel|asparagus`
		),
		'spring-onion'
	),
	R(S('zwiebel|onion|schalotte|shallot|röstzwiebel'), 'onion'),
	R(S('knoblauch|garlic'), 'garlic', true),
	R(S(`(bell|sweet|red|yellow|green|orange|roasted|grilled) peppers?|peppers@E`), 'pepper'),
	R(S('bell ?peppers?|paprika|spitzpaprika|capsicum|peperoni|schmorgurke paprika'), 'pepper'),
	R(
		S(
			'(black |white |ground |cracked |schwarzer |weißer |bunter )?pfeffer|pepper(?!mint)|peppercorn'
		),
		null
	),
	R(S('chili|chilli|chile@E|chiles|peperoncino|jalapeño|jalapeno|habanero|cayenne'), 'chili'),
	R(S('zuckermais|sweetcorn|corn|mais'), 'corn'),
	R(
		S('zuckerschote|zuckererbse|sugar snap|snow pea|schnee ?erbse|edamame|erbse|peas?' + END),
		'pea'
	),
	R(S('kichererbse|chickpea|garbanzo'), 'chickpea'),
	R(S('linse|lentil|dh?al' + END), 'lentil'),
	R(S('bohne|bean(?!sprout)|kidney|borlotti|cannellini|limabohne'), 'bean'),
	R(S('sprossen|sprouts|keimlinge'), 'herb'),
	R(
		A(
			'pilz|champignon|mushroom|shiitake|pfifferling|steinpilz|portobello|egerling|kräuterseitling|austernpilz|enoki|morchel|chanterelle|button mushroom'
		),
		'mushroom'
	),
	R(S('möhre|karotte|carrot|mohrrübe|rübli|wurzelgemüse|baby ?carrot'), 'carrot'),
	R(S('blumenkohl|cauliflower|brokkoli|broccoli|romanesco|rosenkohl|brussels? sprout'), 'broccoli'),
	R(S('grünkohl|kale|spinat|spinach|mangold|chard|palak|brennnessel'), 'spinach'),
	R(
		S(
			"rucola|rocket|arugula|feldsalat|lamb'?s lettuce|(kopf|eisberg|romana|blatt|endivien|chicorée|pflück)?salat|lettuce|romaine|radicchio|pak choi|pak choy|bok choy|mizuna|endive|chicory|blattgemüse|salad leaves"
		),
		'leaf'
	),
	R(S('basilikum|basil'), 'basil'),
	R(
		S(
			'rotkohl|weißkohl|spitzkohl|wirsing|kohlrabi|kohl@E|cabbage|sauerkraut|kimchi|coleslaw|krautsalat|chinakohl|napa'
		),
		'cabbage'
	),
	R(S('avocado|guacamole'), 'avocado'),
	R(S('gurke|cucumber|cornichon|gherkin|kiwi'), 'cucumber'),
	R(S('zucchini|courgette|zucchetti'), 'zucchini'),
	R(S('aubergine|eggplant|melanzane|brinjal'), 'aubergine'),
	R(S('kürbiskern|pumpkin seed'), 'seed'),
	R(S('kürbis|pumpkin|butternut|squash|hokkaido|muskatkürbis'), 'pumpkin'),
	R(S('rote[rn]? bete|rote[rn]? beete|rote rübe|rotebete|beet(root)?|rüben'), 'beet'),
	R(
		S(
			'radieschen|radish|rettich|tomate|tomato|cherrytomate|kirschtomate|datteltomate|strauchtomate|tomaten'
		),
		'tomato'
	),
	R(
		S(
			'süßkartoffel|sweet potato|yam|kartoffel|potato|pommes|fries|kroketten|rösti|bratkartoffel|hash ?brown|erdäpfel'
		),
		'potato'
	),
	// --- herbs ---
	R(
		S(
			'bärlauch|peppermint|petersilie|parsley|dill@E|koriander@E|coriander@E|cilantro|thymian|thyme|rosmarin|rosemary|minze|mint@E|salbei|sage@E|oregano|majoran|marjoram|schnittlauch|chives?|kräuter|herbs?@E|bohnenkraut|estragon|tarragon|liebstöckel|kerbel|basilikum|curryblätter|curry leaves|zitronengras|lemongrass|thai basil|kresse|cress'
		),
		'herb'
	),
	// --- spices (weak) ---
	R(
		S(
			"currypulver|curry|kurkuma|turmeric|kreuzkümmel|cumin|kümmel|zimt|cinnamon|muskat|nutmeg|garam masala|gewürz|spice|seasoning|ras el hanout|za'?atar|sumach?|sumac|koriandersamen|nelken?@E|cloves?@E|kardamom|cardamom|safran|saffron|ingwer|ginger|anis|fenugreek|bockshornklee|piment|allspice|paprika-?gewürz|chinese five|fünf gewürz|dukkah|tajine"
		),
		'sprinkle',
		true
	),
	// --- eggs, dairy, cheese ---
	R(S(`eier@E|ei@E|eigelb|eiweiß|eiklar|egg@E|eggs@E|egg yolk|egg white|spiegelei|rührei`), 'egg'),
	R(S('mozzarella|burrata|bocconcini|stracciatella'), 'mozzarella'),
	R(S('halloumi|grillkäse|tofu|tempeh|paneer|seitan'), 'cube'),
	R(
		A(
			'käse|cheese|gouda|parmesan|parmigiano|pecorino|cheddar|emmentaler?|gruyère|gruyere|edamer|gorgonzola|feta|manchego|raclette|reibekäse|grana padano|camembert|brie@E|tilsiter|appenzeller|bergkäse|fontina|provolone|comté|comte|taleggio|queso|halloumi'
		),
		'cheese'
	),
	// --- meat and fish ---
	R(
		S(
			'hackfleisch|hackfl|(rinder|schweine|gemischte[sr]?|lamm|puten|hähnchen)?hack@E|gehacktes|faschiertes|mince@E|minced|ground (beef|pork|turkey|lamb|meat|chicken)|rindergeschnetzeltes hack|bolognese|beef mince'
		),
		'mince'
	),
	R(S('speck|bacon|pancetta|guanciale|schinkenspeck|frühstücksspeck|bauchspeck'), 'bacon'),
	R(
		S(
			'wurst|würstchen|wiener|frankfurter|sausage|bratwurst|bockwurst|chorizo|salami|mettwurst|cabanossi|knacker|merguez|nduja'
		),
		'sausage'
	),
	R(
		S(
			'schinken|ham@E|prosciutto|serrano|parma|aufschnitt|lachsschinken|mortadella|schnitzel|cutlet|kotelett|rinder|rind(?=fleisch|steak|filet|gulasch|braten|s)|beef|steak|schwein|pork|lamm|lamb|kalb|veal|fleisch|meat@E|braten|roastbeef|gulasch|ribs@E|rippchen|filet|leber|liver|gyros|döner|kebab|pulled|brisket|wild@E|venison|reh@E|hirsch'
		),
		'slice'
	),
	R(
		S(
			'hähnchen|hühnchen|huhn|hühner|chicken|pute@E|puten|turkey|truthahn|geflügel|poultry|drumstick|keule|ente@E|entenbrust|duck|hendl|brathähnchen|gans@E|goose'
		),
		'chicken'
	),
	R(
		S(
			'garnele|shrimp|prawn|scampi|krabbe|crab|langoustine|hummer|lobster|krebs|meeresfrüchte|seafood|calamari|tintenfisch|squid|muschel|mussel|clam|venusmuschel|jakobsmuschel|scallop|oktopus|octopus'
		),
		'prawn'
	),
	R(S('lachs|salmon|forelle|trout|räucherlachs|lox@E|saibling|char@E'), 'salmon'),
	R(
		S(
			'fisch|fish@E|thunfisch|tuna|kabeljau|cod@E|seelachs|pollock|hering|herring|sardine|sardelle|anchov|makrele|mackerel|dorade|bream|zander|barsch|bass@E|seehecht|hake|scholle|plaice|rotbarsch|seafood|heilbutt|halibut|pangasius|tilapia|wels|karpfen|surimi|fischstäbchen|fischfilet'
		),
		'fillet'
	),
	// --- nuts, seeds ---
	R(S('sesam|sesame'), 'sesame'),
	R(
		S(
			'sonnenblumenkern|sunflower seed|chia|leinsamen|flax|mohn@E|poppy|samen@E|seeds?@E|kerne@E|hanfsamen|hemp|pinienkern|pine nut'
		),
		'seed'
	),
	R(A('nuss@E|nüsse|nüssen'), 'nut'),
	R(
		S(
			'nut@E|nuts@E|erdnüss|mandel|almond|walnuss|walnut|cashew|pistazie|pistachio|haselnuss|hazelnut|pecan|erdnuss|peanut|macadamia|marzipan|kokosnuss|schokolade|chocolate|schoko|kuvertüre|nutella|nougat'
		),
		'nut'
	),
	// --- fruit ---
	R(S('banane|banana|plantain'), 'banana'),
	R(S('apfel|äpfel|apple|apfelmus|applesauce'), 'apple'),
	R(
		S(
			'birne|pear@E|pears@E|pfirsich|peach|nektarine|nectarine|pflaume|plum@E|zwetschge|aprikose|apricot|quitte'
		),
		'apple'
	),
	R(
		S('orange|mandarine|clementine|grapefruit|pomelo|blutorange|satsuma|tangerine|bergamotte'),
		'citrus'
	),
	R(S('zitrone|lemon|limette|lime@E|limes@E|yuzu'), 'lemon'),
	R(
		S(
			'mango|ananas|pineapple|papaya|melone|melon|wassermelone|granatapfel|pomegranate|physalis|passion|maracuja'
		),
		'wedge'
	),
	R(
		S(
			'beere|berry|berries|erdbeere|himbeere|blaubeere|heidelbeere|strawberr|raspberr|blueberr|brombeere|blackberr|johannisbeere|currant|cranberr|preiselbeere|kirsche|cherry|cherries|traube|grape|rosine|raisin|sultanine|dattel|date@E|dates@E|feige|fig@E|figs@E|goji|trockenfrüchte|dried fruit'
		),
		'berry'
	),
	R(S('olive|oliven'), 'olive'),
	// --- bread ---
	R(A('brot@E'), 'crouton'),
	R(
		S(
			'bread|toast|brötchen|baguette|bun@E|buns@E|brioche|croissant|semmel|ciabatta|fladenbrot|pita@E|naan|crouton|cracker|zwieback|knäckebrot|focaccia|laugen|bagel|burger bun|sandwich'
		),
		'crouton'
	)
];

/** Sentence parts that carry an ingredient name. */
function parts(raw: string): string[] {
	let t = raw.toLowerCase().replace(/\([^)]*\)/g, ' ');
	t = t.split(/,|;|\s[–—-]\s|:/)[0];
	t = t.split(/\s(?:in|mit|aus|zum|zur|for|with|from|to|für|auf|im|on|as|vom|an)\s/)[0];
	return t
		.split(/\s(?:und|and|oder|or)\s|\s&\s|\//)
		.map((x) => x.trim())
		.filter(Boolean);
}

const GRATED = /gerieben|grated|shred|reibe|gehobelt|shaved/;

/** What the old `dish.ts` drew for a food category. */
const CATEGORY_PIECES: Partial<Record<FoodCategoryView, Piece[]>> = {
	vegetables: ['leaf', 'tomato', 'wedge'],
	herbs: ['herb'],
	fruit: ['lemon', 'berry'],
	cheese: ['cube'],
	dairy: ['dollop'],
	eggs: ['egg'],
	meat: ['slice'],
	fish: ['fillet'],
	grains: ['grain'],
	bread: ['crouton'],
	legumes: ['bean'],
	sauces: ['swoosh'],
	spices: ['sprinkle'],
	nuts: ['nut'],
	sweets: ['berry'],
	leftovers: ['dot'],
	other: ['dot']
};

type Hit = { piece: Piece | null; weak: boolean } | null;

function lookup(part: string, raw: string): Hit {
	for (const r of RULES) {
		if (!r.re.test(part)) continue;
		let piece = r.p;
		if (piece === 'cube' && GRATED.test(raw.toLowerCase())) piece = 'cheese';
		return { piece, weak: r.weak === true };
	}
	return null;
}

/**
 * Pieces for a recipe's ingredients (English and German names, any amounts
 * or adjectives around them: "250 g spinach", "Kichererbsen (Dose)"), in the
 * order given, distinct, at most `max`. Staples that don't show (salt, oil,
 * water, flour, sugar, stock) give nothing. Garlic, spices and sauces only
 * fill what is left (garlic only when fewer than two real pieces). Falls
 * back to `categories` for names it doesn't know.
 */
export function piecesFor(
	ingredients: readonly string[],
	categories: readonly FoodCategoryView[] = [],
	max = 6
): Piece[] {
	const strong: Piece[] = [];
	const weak: Piece[] = [];
	let unknown = 0;
	for (const raw of ingredients) {
		for (const part of parts(raw)) {
			const hit = lookup(part, raw);
			if (!hit) {
				if (/[a-zäöü]{3}/.test(part)) unknown++;
				continue;
			}
			if (!hit.piece) continue;
			const list = hit.weak ? weak : strong;
			if (!strong.includes(hit.piece) && !weak.includes(hit.piece)) list.push(hit.piece);
		}
	}
	const out = strong.slice(0, max);
	if ((unknown > 0 && out.length < 3) || out.length === 0) {
		for (const c of categories) {
			for (const p of CATEGORY_PIECES[c] ?? []) {
				if (out.length < max && !out.includes(p) && !weak.includes(p)) out.push(p);
			}
		}
	}
	// garlic only when the dish would otherwise look bare
	for (const p of weak) {
		if (out.length >= max) break;
		if (p === 'garlic' && out.length >= 2) continue;
		if (!out.includes(p)) out.push(p);
	}
	return out;
}
