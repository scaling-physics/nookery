// The dish library: >= 100 kinds + matching a recipe to one (2026-11-10).
//
// Each kind says how its picture is built (vessel + base + tint + a few
// signature pieces) and which words name it, in English and German. Words are
// written lower case with `|` between alternatives; accents, plurals and
// "ae/oe/ue" spellings are folded by `norm` on both sides, so `käse`,
// `kaese` and `kase` are the same word.
import type { Base, Colour, DishKind, Piece, Vessel } from './types.js';

/** One catalogue line: words are `a|b c|d` (alternatives; phrases have spaces). */
function d(
	id: string,
	label: string,
	words: string,
	vessel: Vessel,
	base: Base,
	tint: Colour,
	signature: Piece[],
	hot: boolean,
	side?: Base,
	sideTint?: Colour
): DishKind {
	const kind: DishKind = { id, label, words: words.split('|'), vessel, base, tint, signature, hot };
	if (side) {
		kind.side = side;
		kind.sideTint = sideTint ?? 'cream';
	}
	return kind;
}

export const DISHES: DishKind[] = [
	// ---- pasta ----
	d(
		'spaghetti-carbonara',
		'Spaghetti carbonara',
		'carbonara|spaghetti carbonara|pasta carbonara',
		'plate',
		'spaghetti',
		'cream',
		['bacon', 'cheese', 'sprinkle'],
		true
	),
	d(
		'spaghetti-bolognese',
		'Spaghetti bolognese',
		'bolognese|bolognaise|spaghetti bolognese|pasta bolognese|ragu|nudeln mit hackfleisch|spaghetti bolo',
		'plate',
		'spaghetti',
		'rust',
		['mince', 'cheese', 'basil'],
		true
	),
	d(
		'aglio-e-olio',
		'Spaghetti aglio e olio',
		'aglio e olio|aglio olio|spaghetti aglio|garlic pasta|garlic spaghetti|knoblauch spaghetti|knoblauchnudeln',
		'plate',
		'spaghetti',
		'wheat',
		['garlic', 'chili', 'herb'],
		true
	),
	d(
		'pesto-pasta',
		'Pesto pasta',
		'pesto|pesto pasta|pesto spaghetti|pesto penne|pasta pesto|pesto nudeln|nudeln mit pesto|pasta al pesto',
		'plate',
		'short-pasta',
		'leaf',
		['basil', 'nut', 'cheese'],
		true
	),
	d(
		'pasta-pomodoro',
		'Tomato pasta',
		'pomodoro|tomato pasta|tomato spaghetti|pasta pomodoro|spaghetti pomodoro|spaghetti napoli|napoli|tomatensauce nudeln|nudeln mit tomatensauce|tomato sauce pasta|pasta al pomodoro',
		'plate',
		'spaghetti',
		'terra',
		['tomato', 'basil', 'cheese'],
		true
	),
	d(
		'penne-arrabbiata',
		'Penne arrabbiata',
		'arrabbiata|penne arrabbiata|spicy pasta|scharfe nudeln',
		'plate',
		'short-pasta',
		'terra',
		['chili', 'garlic', 'herb'],
		true
	),
	d(
		'mushroom-pasta',
		'Creamy mushroom pasta',
		'mushroom pasta|creamy mushroom pasta|pilznudeln|pilz pasta|pasta mit pilzen|champignon pasta|tagliatelle pilz|mushroom tagliatelle',
		'plate',
		'short-pasta',
		'cream',
		['mushroom', 'herb', 'sprinkle'],
		true
	),
	d(
		'mac-and-cheese',
		'Mac and cheese',
		'mac cheese|macaroni cheese|mac and cheese|kasenudeln|kasemakkaroni|makkaroni kase|cheesy pasta',
		'deep',
		'short-pasta',
		'pumpkin',
		['cheese', 'sprinkle'],
		true
	),
	d(
		'lasagne',
		'Lasagne',
		'lasagne|lasagna|lasagne bolognese|gemuselasagne|vegetable lasagne',
		'tray',
		'lasagne',
		'rust',
		['cheese', 'basil'],
		true
	),
	d(
		'gnocchi',
		'Gnocchi',
		'gnocchi|kartoffelgnocchi|gnocchi pfanne|gnocchi pan',
		'pan',
		'gnocchi',
		'cream',
		['tomato', 'herb', 'cheese'],
		true
	),
	d(
		'ravioli',
		'Ravioli',
		'ravioli|tortellini|tortelloni|cappelletti|gefullte nudeln',
		'plate',
		'dumpling',
		'wheat',
		['swoosh', 'basil', 'cheese'],
		true
	),
	d(
		'pasta-salad',
		'Pasta salad',
		'pasta salad|nudelsalat|pasta salat|nudel salat',
		'salad',
		'short-pasta',
		'wheat',
		['tomato', 'cucumber', 'olive', 'cube'],
		false
	),
	d(
		'pasta-bake',
		'Pasta bake',
		'pasta bake|nudelauflauf|penne al forno|baked pasta|pasta al forno|nudel auflauf|ofennudeln',
		'tray',
		'gratin',
		'terra',
		['cheese', 'basil'],
		true
	),
	d(
		'prawn-pasta',
		'Prawn pasta',
		'prawn pasta|shrimp pasta|garnelen pasta|garnelennudeln|seafood pasta|spaghetti frutti di mare|spaghetti vongole|spaghetti scampi|scampi',
		'plate',
		'spaghetti',
		'wheat',
		['prawn', 'garlic', 'lemon', 'herb'],
		true
	),
	d(
		'cacio-e-pepe',
		'Cacio e pepe',
		'cacio e pepe|cacio pepe|spaghetti cacio',
		'plate',
		'spaghetti',
		'cream',
		['cheese', 'sprinkle'],
		true
	),
	d(
		'pasta',
		'Pasta with sauce',
		'pasta|nudeln|nudel|spaghetti|penne|fusilli|tagliatelle|rigatoni|linguine|farfalle|macaroni|makkaroni|noodles with sauce',
		'plate',
		'spaghetti',
		'wheat',
		['herb', 'cheese'],
		true
	),

	// ---- rice ----
	d(
		'risotto',
		'Risotto',
		'risotto|pilzrisotto|mushroom risotto|kurbisrisotto|pumpkin risotto|spargelrisotto',
		'plate',
		'creamy',
		'cream',
		['mushroom', 'herb', 'cheese'],
		true
	),
	d(
		'fried-rice',
		'Fried rice',
		'fried rice|egg fried rice|gebratener reis|gebratenen reis|reispfanne|nasi goreng|bratreis|reis pfanne|special fried rice',
		'wok',
		'fried-rice',
		'wheat',
		['egg', 'spring-onion', 'pea', 'carrot'],
		true
	),
	d(
		'paella',
		'Paella',
		'paella|seafood paella|paella valenciana',
		'pan',
		'rice',
		'lemon',
		['prawn', 'pepper', 'pea', 'lemon'],
		true
	),
	d(
		'biryani',
		'Biryani',
		'biryani|biriyani|birjani',
		'pot',
		'rice',
		'pumpkin',
		['chicken', 'onion', 'herb', 'lemon'],
		true
	),
	d(
		'pilaf',
		'Pilaf',
		'pilaf|pilau|pilav|pulao|reis pilaw|pilaw',
		'pan',
		'rice',
		'wheat',
		['carrot', 'nut', 'herb'],
		true
	),
	d(
		'rice-bowl',
		'Rice bowl',
		'rice bowl|reisschussel|reis bowl|reis schussel|donburi|rice|reis|gyudon|bibimbap',
		'bowl',
		'rice',
		'cream',
		['sesame', 'spring-onion'],
		true
	),
	d(
		'poke-bowl',
		'Poke bowl',
		'poke|poke bowl|sushi bowl|salmon bowl|lachs bowl',
		'bowl',
		'rice',
		'cream',
		['salmon', 'avocado', 'cucumber', 'sesame'],
		false
	),
	d(
		'sushi',
		'Sushi',
		'sushi|maki|sushi rolls|sushi roll|nigiri|futomaki|california roll',
		'sushi',
		'sushi-roll',
		'cream',
		['salmon', 'avocado', 'cucumber', 'sesame'],
		false
	),
	d(
		'onigiri',
		'Onigiri',
		'onigiri|rice ball|reisball|reisbaellchen|reisbalchen',
		'plate',
		'rice',
		'milk',
		['sesame', 'fillet'],
		false
	),
	d(
		'congee',
		'Congee',
		'congee|jook|reisbrei|reissuppe|rice porridge',
		'deep',
		'porridge',
		'milk',
		['egg', 'spring-onion', 'sesame'],
		true
	),
	d(
		'rice-bake',
		'Rice bake',
		'rice bake|rice casserole|reisauflauf|ofenreis|reis auflauf|leftover rice bake|baked rice',
		'tray',
		'gratin',
		'wheat',
		['cheese', 'pea', 'herb'],
		true
	),
	d(
		'jollof-rice',
		'Jollof rice',
		'jollof|jollof rice|tomato rice|tomatenreis|reis mit tomaten',
		'pot',
		'rice',
		'rust',
		['pepper', 'tomato', 'chili'],
		true
	),

	// ---- curries ----
	d(
		'chickpea-curry',
		'Chickpea curry',
		'chickpea curry|chana masala|kichererbsen curry|kichererbsencurry|chana curry|channa masala|kichererbsen masala',
		'deep',
		'thick',
		'pumpkin',
		['chickpea', 'herb', 'chili'],
		true,
		'rice'
	),
	d(
		'chicken-curry',
		'Chicken curry',
		'chicken curry|curry chicken|hahnchen curry|hahnchencurry|huhnchen curry|huhnercurry|chicken madras|chicken jalfrezi|hahnchen currypfanne',
		'plate',
		'rice-side',
		'pumpkin',
		['chicken', 'herb', 'onion'],
		true
	),
	d(
		'butter-chicken',
		'Butter chicken',
		'butter chicken|chicken tikka|tikka masala|chicken tikka masala|murgh makhani',
		'plate',
		'rice-side',
		'rust',
		['chicken', 'herb', 'dollop'],
		true
	),
	d(
		'dal',
		'Dal',
		'dal|dhal|daal|dahl|red lentil dal|linsen dal|linsencurry|lentil curry|tadka dal|rote linsen curry|linsen curry',
		'deep',
		'thick',
		'lemon',
		['lentil', 'herb', 'chili'],
		true,
		'rice'
	),
	d(
		'korma',
		'Korma',
		'korma|chicken korma|vegetable korma|gemuse korma|kokoscurry|coconut curry',
		'deep',
		'creamy',
		'cream',
		['nut', 'chicken', 'herb'],
		true,
		'rice'
	),
	d(
		'thai-green-curry',
		'Thai green curry',
		'thai green curry|green curry|grunes curry|thai curry|thai gemusecurry|thai chicken curry',
		'deep',
		'thick',
		'leaf',
		['basil', 'chili', 'zucchini'],
		true,
		'rice'
	),
	d(
		'thai-red-curry',
		'Thai red curry',
		'thai red curry|red curry|rotes curry|rotes thai curry|panang curry|massaman curry|massaman|panang',
		'deep',
		'thick',
		'terra',
		['pepper', 'basil', 'chili'],
		true,
		'rice'
	),
	d(
		'katsu-curry',
		'Katsu curry',
		'katsu curry|japanese curry|japanisches curry|chicken katsu|katsu|curry reis|kare raisu',
		'plate',
		'rice-side',
		'brown',
		['chicken', 'carrot'],
		true
	),
	d(
		'saag-paneer',
		'Saag paneer',
		'saag paneer|palak paneer|spinach paneer|paneer curry|spinach curry|spinatcurry|spinat curry|palak',
		'deep',
		'creamy',
		'leaf',
		['spinach', 'cube', 'chili'],
		true,
		'rice'
	),
	d(
		'vegetable-curry',
		'Vegetable curry',
		'curry|vegetable curry|veggie curry|gemusecurry|gemuse curry|sweet potato curry|susskartoffel curry|pumpkin curry|kurbis curry|kurbiscurry|currypfanne',
		'deep',
		'thick',
		'pumpkin',
		['carrot', 'pea', 'pumpkin', 'herb'],
		true,
		'rice'
	),

	// ---- soups and stews ----
	d(
		'tomato-soup',
		'Tomato soup',
		'tomato soup|tomatensuppe|tomatencremesuppe|tomaten suppe|tomatencreme suppe',
		'deep',
		'creamy',
		'terra',
		['herb', 'crouton', 'dollop'],
		true
	),
	d(
		'pumpkin-soup',
		'Pumpkin soup',
		'pumpkin soup|kurbissuppe|squash soup|butternut soup|hokkaido suppe|hokkaidosuppe|butternut squash soup|susskartoffelsuppe',
		'deep',
		'creamy',
		'pumpkin',
		['seed', 'dollop', 'herb'],
		true
	),
	d(
		'carrot-soup',
		'Carrot soup',
		'carrot soup|karottensuppe|mohrensuppe|karottencremesuppe|carrot ginger soup|moehrensuppe|karotten suppe',
		'deep',
		'creamy',
		'pumpkin',
		['carrot', 'seed', 'dollop'],
		true
	),
	d(
		'minestrone',
		'Minestrone',
		'minestrone|gemusesuppe|vegetable soup|gemuse suppe|veggie soup',
		'deep',
		'broth',
		'terra',
		['carrot', 'pea', 'bean', 'herb'],
		true
	),
	d(
		'ramen',
		'Ramen',
		'ramen|shoyu ramen|miso ramen|tonkotsu|ramen noodles|udon|udon soup|nudelsuppe asiatisch',
		'deep',
		'ramen',
		'wheat',
		['egg', 'spring-onion', 'sesame'],
		true
	),
	d(
		'pho',
		'Pho',
		'pho|pho bo|pho ga|vietnamese noodle soup|vietnamesische nudelsuppe',
		'deep',
		'ramen',
		'brown',
		['herb', 'chili', 'slice'],
		true
	),
	d(
		'lentil-soup',
		'Lentil soup',
		'lentil soup|linsensuppe|rote linsensuppe|red lentil soup|linsen suppe',
		'deep',
		'thick',
		'terra',
		['lentil', 'carrot', 'herb'],
		true
	),
	d(
		'lentil-eintopf',
		'Linseneintopf',
		'linseneintopf|lentil stew|linsen eintopf|linsentopf|linsen und spatzle',
		'pot',
		'thick',
		'brown',
		['lentil', 'carrot', 'sausage'],
		true
	),
	d(
		'potato-soup',
		'Potato soup',
		'potato soup|kartoffelsuppe|potato leek soup|leek soup|lauchsuppe|kartoffel lauch suppe|kartoffel suppe|loaded potato soup',
		'deep',
		'creamy',
		'cream',
		['potato', 'spring-onion', 'bacon'],
		true
	),
	d(
		'goulash',
		'Goulash',
		'goulash|gulasch|gulaschsuppe|beef goulash|rindergulasch|szegediner|paprikagulasch|gulasch suppe',
		'pot',
		'thick',
		'rust',
		['pepper', 'onion', 'dollop'],
		true
	),
	d(
		'chili-con-carne',
		'Chili con carne',
		'chili|chilli|chili con carne|chilli con carne|chili sin carne|bohnen chili|bean chili|bohnenchili|veggie chili',
		'deep',
		'thick',
		'terra',
		['bean', 'mince', 'corn', 'chili'],
		true,
		'rice'
	),
	d(
		'eintopf',
		'Eintopf',
		'eintopf|gemuse eintopf|gemuseeintopf|stew|hearty stew|kartoffeleintopf|kartoffel eintopf|bohneneintopf|stew with dumplings|topf|one pot',
		'pot',
		'thick',
		'brown',
		['carrot', 'potato', 'sausage', 'herb'],
		true
	),
	d(
		'borscht',
		'Borscht',
		'borscht|borsch|borschtsch|rote bete suppe|beetroot soup|rote beete suppe|beet soup|rote bete eintopf',
		'deep',
		'broth',
		'plum',
		['beet', 'dollop', 'herb'],
		true
	),
	d(
		'miso-soup',
		'Miso soup',
		'miso soup|misosuppe|miso|miso suppe',
		'mug',
		'broth',
		'brown',
		['cube', 'spring-onion'],
		true
	),
	d(
		'chicken-soup',
		'Chicken soup',
		'chicken soup|huhnersuppe|chicken noodle soup|huhnchensuppe|hahnchensuppe|nudelsuppe|huhner nudelsuppe|chicken broth|brathe',
		'deep',
		'broth',
		'wheat',
		['chicken', 'carrot', 'herb'],
		true
	),
	d(
		'mushroom-soup',
		'Mushroom soup',
		'mushroom soup|pilzsuppe|champignoncremesuppe|cream of mushroom|champignonsuppe|pilzcremesuppe|pilz suppe',
		'mug',
		'creamy',
		'brown',
		['mushroom', 'herb', 'crouton'],
		true
	),
	d(
		'pea-soup',
		'Pea soup',
		'pea soup|erbsensuppe|split pea soup|erbseneintopf|erbsen suppe|erbsencremesuppe',
		'deep',
		'creamy',
		'pea',
		['pea', 'bacon', 'crouton'],
		true
	),
	d(
		'broccoli-soup',
		'Broccoli soup',
		'broccoli soup|brokkolisuppe|brokkoli cremesuppe|brokkolicremesuppe|broccoli cheddar soup|brokkoli suppe',
		'mug',
		'creamy',
		'leaf',
		['broccoli', 'cheese'],
		true
	),
	d(
		'onion-soup',
		'Onion soup',
		'onion soup|zwiebelsuppe|french onion soup|zwiebel suppe',
		'pot',
		'broth',
		'brown',
		['onion', 'crouton', 'cheese'],
		true
	),
	d(
		'gazpacho',
		'Gazpacho',
		'gazpacho|cold soup|kaltschale|kalte suppe|salmorejo',
		'glass',
		'smoothie',
		'terra',
		['cucumber', 'herb', 'tomato'],
		false
	),
	d(
		'soup',
		'Soup',
		'soup|suppe|broth|bruhe|cremesuppe|cream soup|veloute',
		'deep',
		'broth',
		'wheat',
		['herb'],
		true
	),

	// ---- eggs ----
	d(
		'shakshuka',
		'Shakshuka',
		'shakshuka|shakshouka|chakchouka|menemen|eggs in tomato sauce|eier in tomatensauce|eggs in purgatory',
		'pan',
		'thick',
		'terra',
		['fried-egg', 'herb'],
		true
	),
	d(
		'omelette',
		'Omelette',
		'omelette|omelet|omelett|omlett|spinach omelette|cheese omelette|cheese omelet|western omelette',
		'pan',
		'omelette',
		'lemon',
		['herb'],
		true
	),
	d(
		'frittata',
		'Frittata',
		'frittata|spanish tortilla|tortilla de patatas|spanische tortilla|eierkuchen herzhaft|egg bake|egg muffins',
		'pan',
		'omelette',
		'lemon',
		['zucchini', 'herb', 'tomato'],
		true
	),
	d(
		'scrambled-eggs',
		'Scrambled eggs',
		'scrambled eggs|scrambled egg|ruhrei|ruhreier|rührei|rueherei|egg scramble|tofu scramble|ruhrei mit|scrambled',
		'plate',
		'scramble',
		'lemon',
		['spring-onion', 'tomato', 'herb'],
		true
	),
	d(
		'fried-eggs',
		'Fried eggs',
		'fried egg|fried eggs|spiegelei|spiegeleier|sunny side up|setzei|bauernfruhstuck|bauernfruehstueck|farmers breakfast|full english|english breakfast',
		'plate',
		'fried-egg',
		'milk',
		['herb', 'tomato', 'bacon'],
		true
	),
	d(
		'eggs-on-toast',
		'Fried eggs on toast',
		'fried egg toast|egg on toast|eggs on toast|egg toast|spiegelei toast|spiegelei brot|spiegeleibrot|ei toast|poached egg toast|egg and toast',
		'board',
		'toast',
		'wheat',
		['fried-egg', 'herb'],
		true
	),
	d(
		'strammer-max',
		'Strammer Max',
		'strammer max|strammer|schinkenbrot mit spiegelei',
		'board',
		'toast',
		'brown',
		['slice', 'fried-egg', 'onion'],
		true
	),
	d(
		'eggs-benedict',
		'Eggs Benedict',
		'eggs benedict|egg benedict|benedict|poached eggs|poached egg|pochierte eier|pochiertes ei|eggs florentine',
		'plate',
		'toast',
		'wheat',
		['egg', 'spinach', 'swoosh'],
		true
	),

	// ---- breakfast ----
	d(
		'porridge',
		'Porridge',
		'porridge|haferbrei|oatmeal|haferflocken|haferflockenbrei|oat porridge|grutze|hirsebrei|buckwheat porridge|griesbrei|grieskoch|hafer porridge',
		'small',
		'porridge',
		'wheat',
		['swoosh', 'berry', 'nut'],
		true
	),
	d(
		'overnight-oats',
		'Overnight oats',
		'overnight oats|overnight oat|ubernacht hafer|overnight hafer|overnight haferflocken|bircher|bircher muesli|birchermusli',
		'jar',
		'porridge',
		'cream',
		['berry', 'nut', 'banana'],
		false
	),
	d(
		'pancakes',
		'Pancakes',
		'pancakes|pancake|pfannkuchen|eierkuchen|crepes|crepe|palatschinken|crepes suzette|pfannkuchen mit|american pancakes|blini|blinis',
		'stack',
		'pancake',
		'brown',
		['berry', 'swoosh', 'banana'],
		true
	),
	d(
		'waffles',
		'Waffles',
		'waffles|waffle|waffeln|belgian waffles|belgische waffeln|herzwaffeln',
		'stack',
		'pancake',
		'wheat',
		['berry', 'swoosh', 'sprinkle'],
		true
	),
	d(
		'french-toast',
		'French toast',
		'french toast|arme ritter|armer ritter|eggy bread|pain perdu|brioche toast|zimttoast|cinnamon toast',
		'plate',
		'toast',
		'pumpkin',
		['berry', 'banana', 'swoosh'],
		true
	),
	d(
		'yoghurt-bowl',
		'Yoghurt bowl',
		'yoghurt bowl|yogurt bowl|joghurt bowl|skyr|skyr bowl|quark bowl|quark|joghurt mit fruchten|frühstücksbowl|fruhstucksbowl|breakfast bowl|joghurt schussel|yogurt|joghurt',
		'small',
		'yoghurt',
		'milk',
		['berry', 'nut', 'seed'],
		false
	),
	d(
		'parfait',
		'Parfait',
		'parfait|yoghurt parfait|skyr parfait|trifle|schichtdessert|dessert im glas|granola parfait|layered dessert',
		'glass',
		'yoghurt',
		'milk',
		['berry', 'nut'],
		false
	),
	d(
		'smoothie',
		'Smoothie',
		'smoothie|shake|milkshake|lassi|mango lassi|protein shake|green smoothie|gruner smoothie|saft',
		'glass',
		'smoothie',
		'pink',
		['berry', 'banana'],
		false
	),
	d(
		'smoothie-bowl',
		'Smoothie bowl',
		'smoothie bowl|acai bowl|smoothie schussel|smoothiebowl|acai|pitaya bowl',
		'bowl',
		'smoothie',
		'pink',
		['berry', 'banana', 'seed', 'nut'],
		false
	),
	d(
		'muesli',
		'Muesli',
		'muesli|musli|cereal|cornflakes|cereals|fruhstucksflocken|knuspermuesli|muesli bowl|musli bowl|corn flakes|cereal bowl',
		'small',
		'grain',
		'wheat',
		['berry', 'nut', 'banana'],
		false
	),
	d(
		'granola',
		'Granola',
		'granola|knuspermusli|crunchy granola|granola bars|granola riegel|energy balls|energiekugeln|knusper',
		'jar',
		'crumble',
		'wheat',
		['nut', 'berry'],
		false
	),

	// ---- bread, toast, pizza ----
	d(
		'bruschetta',
		'Bruschetta',
		'bruschetta|crostini|tomato bruschetta|tomaten bruschetta|tomaten brot|tomato toast',
		'board',
		'toast',
		'wheat',
		['tomato', 'basil', 'garlic'],
		false
	),
	d(
		'sandwich',
		'Sandwich',
		'sandwich|sandwiches|belegtes brot|belegte brote|stulle|club sandwich|panini|baguette sandwich|sub|belegte brotchen|belegtes brotchen|tramezzino|sandwich mit',
		'bun',
		'toast',
		'wheat',
		['cheese', 'slice', 'leaf', 'tomato'],
		false
	),
	d(
		'grilled-cheese',
		'Grilled cheese',
		'grilled cheese|toastie|toasty|kasetoast|croque monsieur|croque madame|cheese toastie|kase toast|ham and cheese toast|schinken kase toast|toast hawaii|melt|tuna melt|cheese toast',
		'board',
		'toast',
		'wheat',
		['cheese', 'slice'],
		true
	),
	d(
		'avocado-toast',
		'Avocado toast',
		'avocado toast|avocado bread|avocadobrot|avocado brot|avo toast|avocado auf toast|guacamole toast',
		'board',
		'toast',
		'wheat',
		['avocado', 'tomato', 'chili', 'seed'],
		false
	),
	d(
		'toast',
		'Toast with toppings',
		'toast|brot|bread with|butterbrot|open sandwich|smorrebrod|bruschetta brot|belegt|kasebrot|schinkenbrot|toasted bread|brotzeit|abendbrot|brotchen|semmel',
		'board',
		'toast',
		'wheat',
		['cheese', 'herb'],
		false
	),
	d(
		'bagel',
		'Bagel',
		'bagel|bagels|lox bagel|salmon bagel|lachs bagel|bagel with|cream cheese bagel|bialy',
		'bun',
		'toast',
		'wheat',
		['dollop', 'salmon', 'cucumber'],
		false
	),
	d(
		'flammkuchen',
		'Flammkuchen',
		'flammkuchen|tarte flambee|elsasser flammkuchen|flammekueche|flamm kuchen|flammkuchen mit',
		'round',
		'flatbread',
		'cream',
		['onion', 'bacon', 'herb'],
		true
	),
	d(
		'margherita',
		'Pizza margherita',
		'margherita|pizza margherita|pizza margarita|margarita|pizza bianca',
		'round',
		'pizza',
		'terra',
		['mozzarella', 'basil', 'tomato'],
		true
	),
	d(
		'pizza-salami',
		'Salami pizza',
		'salami pizza|pizza salami|pepperoni pizza|pizza pepperoni|diavola|pizza diavola|pizza hawaii|pizza prosciutto|pizza schinken|pizza funghi',
		'round',
		'pizza',
		'terra',
		['sausage', 'cheese', 'pepper'],
		true
	),
	d(
		'pizza',
		'Pizza',
		'pizza|calzone|pizzabrotchen|pizza mit|pizzaschnecken|pizza toast|pizzateig|pizzabrot|stone baked pizza|tarte flambe',
		'round',
		'pizza',
		'terra',
		['cheese', 'tomato'],
		true
	),
	d(
		'focaccia',
		'Focaccia',
		'focaccia|focaccia bread|ciabatta|olive bread|olivenbrot|fladenbrot belegt|schiacciata|pinsa',
		'round',
		'flatbread',
		'wheat',
		['herb', 'olive', 'tomato'],
		true
	),
	d(
		'flatbread',
		'Flatbread',
		'flatbread|fladenbrot|naan|pita|pitta|chapati|roti|paratha|lahmacun|pide|turkish bread|tuerkisches fladenbrot|tortilla bread|farinata|socca|piadina|flat bread',
		'round',
		'flatbread',
		'wheat',
		['herb', 'sprinkle'],
		true
	),
	d(
		'bread',
		'Fresh bread',
		'bread|loaf|brotlaib|sourdough|sauerteigbrot|sauerteig|bread loaf|vollkornbrot|zopf|hefezopf|brioche|baguette|soda bread|brot backen|laugenbrot|bread rolls|rolls',
		'loaf',
		'cake',
		'wheat',
		[],
		false
	),
	d(
		'garlic-bread',
		'Garlic bread',
		'garlic bread|knoblauchbrot|krauterbaguette|krauterbrot|herb bread|baguette mit knoblauch|cheesy garlic bread|knoblauch baguette|pull apart bread|garlic baguette',
		'loaf',
		'toast',
		'wheat',
		['garlic', 'herb'],
		true
	),
	d(
		'hummus',
		'Hummus and dips',
		'hummus|houmous|dip|dips|mezze|meze|baba ganoush|tzatziki|guacamole|tapenade|antipasti|antipasto|vorspeisenplatte|aufstrich|spread|labneh|muhammara|mezze platter',
		'small',
		'thick',
		'cream',
		['chickpea', 'swoosh', 'sprinkle'],
		false
	),

	// ---- wraps and handheld ----
	d(
		'burrito',
		'Burrito',
		'burrito|burritos|breakfast burrito|bean burrito|black bean burrito|chimichanga|burrito mit',
		'wrap',
		'tortilla',
		'wheat',
		['bean', 'cheese', 'avocado'],
		true
	),
	d(
		'burrito-bowl',
		'Burrito bowl',
		'burrito bowl|burrito schussel|mexican bowl|mexikanische bowl|taco bowl|taco schussel|tex mex bowl|mexican rice|mexikanischer reis|mexican rice bowl',
		'bowl',
		'rice',
		'cream',
		['bean', 'corn', 'avocado', 'tomato'],
		true
	),
	d(
		'tacos',
		'Tacos',
		'taco|tacos|taco shells|hackfleisch tacos|fish tacos|fisch tacos|tacos mit|street tacos|carnitas|tostada|tostadas',
		'taco',
		'tortilla',
		'wheat',
		['mince', 'tomato', 'leaf', 'cheese'],
		true
	),
	d(
		'quesadilla',
		'Quesadilla',
		'quesadilla|quesadillas|quesadilla mit|tortilla pizza|tortilla toast|cheese quesadilla|kase quesadilla',
		'round',
		'tortilla',
		'wheat',
		['cheese', 'pepper', 'herb'],
		true
	),
	d(
		'gyros',
		'Gyros',
		'gyros|gyros pita|gyros teller|gyrosteller|gyros pfanne|souvlaki wrap|schweinegyros|gyros mit|chicken gyros|pita gyros',
		'wrap',
		'tortilla',
		'wheat',
		['slice', 'onion', 'tomato', 'dollop'],
		true
	),
	d(
		'doner',
		'Döner',
		'doner|doner kebab|durum|durum doner|donerteller|doner teller|shawarma|chicken shawarma|doner tasche|doener|hahnchen doner|kebab wrap|kebabbrot',
		'wrap',
		'tortilla',
		'wheat',
		['slice', 'cabbage', 'onion', 'tomato'],
		true
	),
	d(
		'falafel',
		'Falafel wrap',
		'falafel|falafel wrap|falafel pita|falafel teller|falafel bowl|falafel burger|felafel|falafelteller',
		'wrap',
		'flatbread',
		'wheat',
		['chickpea', 'cucumber', 'tomato', 'dollop'],
		true
	),
	d(
		'fajitas',
		'Fajitas',
		'fajita|fajitas|chicken fajitas|fajita pfanne|steak fajitas|sizzling|veggie fajitas|gemuse fajitas',
		'pan',
		'traybake',
		'terra',
		['pepper', 'onion', 'chicken'],
		true,
		'tortilla',
		'wheat'
	),
	d(
		'enchiladas',
		'Enchiladas',
		'enchilada|enchiladas|enchilada auflauf|cheese enchiladas|chicken enchiladas|tortilla auflauf|tortilla casserole|bean enchiladas',
		'tray',
		'gratin',
		'terra',
		['cheese', 'bean', 'herb'],
		true
	),
	d(
		'wrap',
		'Wrap',
		'wrap|wraps|rolle|rolls with|veggie wrap|chicken wrap|hahnchen wrap|tortilla wrap|tortillas|tortilla|lavash|gefullte tortilla|wrap mit|summer rolls|sommerrollen|reispapierrollen|rice paper rolls',
		'wrap',
		'tortilla',
		'wheat',
		['leaf', 'tomato', 'cucumber'],
		false
	),

	// ---- mains ----
	d(
		'schnitzel',
		'Schnitzel',
		'schnitzel|wiener schnitzel|chicken schnitzel|pork schnitzel|cordon bleu|hahnchenschnitzel|putenschnitzel|schweineschnitzel|jagerschnitzel|zigeunerschnitzel|paniertes schnitzel|breaded chicken|crispy chicken|chicken nuggets|nuggets|panierte',
		'plate',
		'fillet',
		'brown',
		['lemon', 'herb'],
		true,
		'potatoes',
		'wheat'
	),
	d(
		'roast-chicken',
		'Roast chicken',
		'roast chicken|roasted chicken|brathahnchen|hahnchen|hendl|huhn|chicken|roast hen|ofenhahnchen|hahnchenkeulen|chicken legs|chicken thighs|chicken wings|chicken drumsticks|brathuhn|hahnchenschenkel|backhendl|honey chicken|lemon chicken|zitronenhuhn|zitronenhahnchen',
		'plate',
		'roast',
		'brown',
		['herb', 'lemon'],
		true,
		'potatoes',
		'wheat'
	),
	d(
		'chicken-breast',
		'Chicken breast',
		'chicken breast|grilled chicken|hahnchenbrust|putenbrust|turkey|pute|turkey breast|hahnchenfilet|hahnchen filet|pan fried chicken|gebratene hahnchenbrust|puten steak|putensteak|chicken fillet|chicken steak',
		'plate',
		'fillet',
		'pumpkin',
		['herb', 'lemon'],
		true,
		'greens',
		'leaf'
	),
	d(
		'roast-beef',
		'Roast joint',
		'roast beef|braten|schweinebraten|krustenbraten|pot roast|sunday roast|roast pork|rinderbraten|ofenbraten|roast lamb|lammbraten|lamb|lamm|roast|pulled pork|schulterbraten|tafelspitz|lammkeule|braised beef|schmorbraten|kassler|kasseler|schweinefilet|pork roast|pork fillet',
		'plate',
		'roast',
		'dark',
		['slice', 'carrot', 'herb'],
		true,
		'potatoes',
		'wheat'
	),
	d(
		'salmon',
		'Salmon fillet',
		'salmon|lachs|salmon fillet|baked salmon|lachsfilet|ofenlachs|gebratener lachs|grilled salmon|pan fried salmon|lachs mit|salmon with|salmon teriyaki|lachs steak|zitronenlachs|honey salmon|forelle lachs',
		'plate',
		'fillet',
		'pink',
		['lemon', 'herb'],
		true,
		'potatoes',
		'wheat'
	),
	d(
		'fish-and-chips',
		'Fish and chips',
		'fish chips|fish and chips|fish n chips|backfisch pommes|fischstabchen pommes|fish fingers|fischstabchen|backfisch|fish finger sandwich|beer battered fish|crispy fish',
		'plate',
		'fillet',
		'pumpkin',
		['lemon', 'pea'],
		true,
		'fries',
		'wheat'
	),
	d(
		'fish',
		'Fish fillet',
		'fish|fisch|white fish|cod|kabeljau|seelachs|haddock|pan fried fish|fish fillet|fischfilet|trout|forelle|tilapia|seabass|sea bass|wolfsbarsch|dorade|seebarsch|zander|pangasius|tuna steak|thunfischsteak|fish with|fisch mit|rotbarsch|scholle|hering|herring|makrele|mackerel|sardines|sardinen|ofenfisch|steamed fish',
		'plate',
		'fillet',
		'wheat',
		['lemon', 'herb'],
		true,
		'potatoes',
		'wheat'
	),
	d(
		'burger',
		'Burger',
		'burger|hamburger|cheeseburger|beef burger|hackfleisch burger|smash burger|burger mit|chicken burger|hahnchenburger|pulled pork burger|slider|sliders',
		'bun',
		'patty',
		'brown',
		['cheese', 'tomato', 'leaf', 'onion'],
		true
	),
	d(
		'veggie-burger',
		'Veggie burger',
		'veggie burger|bean burger|vegetarian burger|bohnenburger|gemuseburger|vegan burger|halloumi burger|beyond burger|linsenburger|lentil burger|kichererbsenburger|chickpea burger|vegetarischer burger|veggieburger|plant burger',
		'bun',
		'patty',
		'olive',
		['leaf', 'tomato', 'avocado'],
		true
	),
	d(
		'meatballs',
		'Meatballs',
		'meatballs|meatball|frikadellen|frikadelle|hackballchen|fleischpflanzerl|kofte|hackfleischballchen|bouletten|buletten|fleischbaellchen|meatballs in tomato sauce|kottbullar|albondigas|fleischklopse|hackbaellchen|spaghetti meatballs|meatball sub|fleischbällchen',
		'plate',
		'patty',
		'brown',
		['tomato', 'herb', 'swoosh'],
		true,
		'mash',
		'cream'
	),
	d(
		'sausage-mash',
		'Sausages with mash',
		'sausage mash|sausages mash|bangers mash|bangers|sausage|wurst|wurstchen|bratwurst|bratwurst mit|sausage with|wiener|weisswurst|frankfurter|wienerle|sausage casserole|wurstpfanne|bockwurst|bratwurstpfanne|hot dog|hotdog|chorizo|merguez|cevapcici|wurstel',
		'plate',
		'mash',
		'cream',
		['sausage', 'onion', 'swoosh'],
		true
	),
	d(
		'mashed-potatoes',
		'Mashed potatoes',
		'mash|mashed potatoes|mashed potato|kartoffelpuree|kartoffelbrei|stampfkartoffeln|kartoffelstampf|stampf|colcannon|champ|puree|pommes puree|kartoffelmus|susskartoffelpuree|mashed sweet potato',
		'plate',
		'mash',
		'cream',
		['herb', 'swoosh'],
		true
	),
	d(
		'steak',
		'Steak',
		'steak|rumpsteak|ribeye|sirloin|beef steak|entrecote|filet mignon|hufsteak|steak frites|rinderfilet|beef fillet|rib eye|t bone|flank steak|rindersteak|rinder steak|filetsteak|schweinesteak|nackensteak|beef strips|geschnetzeltes|zurcher geschnetzeltes|beef medallions|medaillons|lammkoteletts|lamb chops|lammkotelett|steak mit|bavette|tri tip|flat iron',
		'plate',
		'fillet',
		'dark',
		['garlic', 'herb', 'mushroom'],
		true,
		'fries',
		'wheat'
	),
	d(
		'pork-chop',
		'Pork chop',
		'pork chop|pork chops|schweinekotelett|kotelett|koteletts|schweinekoteletts|karree|kasslerkotelett|jagerbraten|pork cutlet|schnitzel natur|spareribs|ribs|rippchen|bbq ribs|haxe|schweinshaxe|schweinehaxe|eisbein|pork belly|schweinebauch|krustenbauch|hackbraten dinner',
		'plate',
		'fillet',
		'brown',
		['herb', 'onion'],
		true,
		'potatoes',
		'wheat'
	),
	d(
		'kebab',
		'Kebab skewers',
		'skewers|skewer|kebab|spiess|spiesse|schaschlik|souvlaki|shish|chicken skewers|spiesschen|grillspiesse|shish kebab|hahnchenspiesse|gemusespiesse|veggie skewers|fleischspiesse|grillspieß|lamb skewers|adana|shashlik|yakitori|tikka|seekh kebab|gemuse spiesse|hahnchen spiesse',
		'skewer',
		'none',
		'brown',
		['pepper', 'onion', 'chicken'],
		true
	),
	d(
		'satay',
		'Satay',
		'satay|sate|chicken satay|erdnuss spiess|satay spiesse|peanut chicken|erdnusshahnchen|erdnusssauce|peanut sauce|satay chicken|satay skewers|saté',
		'skewer',
		'none',
		'brown',
		['chicken', 'nut', 'swoosh'],
		true
	),
	d(
		'stir-fry',
		'Vegetable stir-fry',
		'stir fry|stirfry|wok gemuse|wok|wokgemuse|gemusepfanne|pfannengericht|gemuse pfanne|asiatische pfanne|veggie stir fry|stir fried vegetables|chicken stir fry|beef stir fry|asian stir fry|tofu stir fry|kung pao|sweet and sour|sweet sour|susssauer|susssaure|szechuan|schezwan|cashew chicken|beef and broccoli|honey soy chicken|hahnchenpfanne|hahnchen pfanne|geflügelpfanne|hackfleischpfanne|hack pfanne|reispfanne mit gemuse|tofu pfanne|gemuse wok|pak choi pfanne|asia pfanne|asiapfanne|pfanne',
		'wok',
		'traybake',
		'brown',
		['pepper', 'broccoli', 'carrot', 'sesame'],
		true,
		'rice',
		'cream'
	),
	d(
		'pad-thai',
		'Pad Thai',
		'pad thai|pad see ew|pad kra pao|thai noodles|thai nudeln|phad thai|thai peanut noodles|erdnussnudeln|peanut noodles|thai wok|glass noodles|glasnudeln|reisnudeln|rice noodles',
		'wok',
		'noodles',
		'wheat',
		['egg', 'nut', 'lemon', 'spring-onion'],
		true
	),
	d(
		'teriyaki',
		'Teriyaki',
		'teriyaki|teriyaki chicken|teriyaki salmon|hahnchen teriyaki|teriyaki lachs|teriyaki beef|chicken teriyaki|yakiniku|yaki|sukiyaki|oyakodon|karaage|japanisches hahnchen|honey garlic chicken|sticky chicken|glazed chicken|glazed salmon|miso glazed|miso lachs|miso salmon|miso aubergine|nasu dengaku',
		'plate',
		'rice-side',
		'brown',
		['chicken', 'sesame', 'spring-onion'],
		true
	),
	d(
		'chow-mein',
		'Chow mein',
		'chow mein|lo mein|gebratene nudeln|bami goreng|mie goreng|fried noodles|bratnudeln|stir fried noodles|nudelpfanne|nudel pfanne|noodle stir fry|yakisoba|noodle|noodles|asian noodles|asia nudeln|asia nudelpfanne|bami|mie|nudeln gebraten|hokkien|singapore noodles|japchae|soba|sesame noodles|sesamnudeln|cold noodles|kalte nudeln|dan dan|dan dan noodles|peanut noodle|erdnuss nudeln|nudel wok|wok nudeln|wok noodles|veggie noodles|gemuse nudeln|gemusenudeln',
		'wok',
		'noodles',
		'brown',
		['carrot', 'cabbage', 'spring-onion', 'chicken'],
		true
	),
	d(
		'dumplings',
		'Dumplings',
		'dumplings|dumpling|gyoza|jiaozi|potstickers|potsticker|dim sum|wontons|wonton|teigtaschen|mandu|pierogi|momos|momo|pelmeni|manti|xiaolongbao|siu mai|shumai|har gow|wan tan|wantan|wonton suppe|wonton soup|varenyky|vareniki|empanadas|empanada|samosa|samosas|piroggen|pirogge|pirozhki|samsa|gefullte teigtaschen|jiaozi dumplings',
		'steamer',
		'dumpling',
		'cream',
		['spring-onion', 'sesame', 'chili'],
		true
	),
	d(
		'bao',
		'Bao buns',
		'bao|baozi|bao buns|bao bun|steamed buns|gua bao|hirata buns|hirata|mantou|steamed bun|dampfbrotchen|hefeklosse|dampfnudeln|dampfnudel|char siu bao|pork buns|pork bun|bao burger|bao mit|filled buns|gefullte brotchen',
		'steamer',
		'dumpling',
		'milk',
		['slice', 'cucumber', 'swoosh'],
		true
	),
	d(
		'spring-rolls',
		'Spring rolls',
		'spring rolls|spring roll|fruhlingsrollen|fruhlingsrolle|egg rolls|egg roll|lumpia|loempia|frühlingsrollen|crispy rolls|knusprige rollen|vegetable rolls|gemuserollen|samosa rolls|summer roll|nem|cha gio|chả giò|rolls fried|filo rolls|filorollen|bricks|brik|bourek|burek|borek|boerek|filo|phyllo|strudel|apfelstrudel|gemusestrudel|spinatstrudel|strudel mit',
		'plate',
		'patty',
		'pumpkin',
		['swoosh', 'chili', 'herb'],
		true
	),
	d(
		'meatloaf',
		'Meatloaf',
		'meatloaf|hackbraten|falscher hase|meat loaf|turkey meatloaf|hackbraten mit|hackbraten im ofen|nut roast|nussbraten|lentil loaf|linsenbraten|linsenbrot|gehacktesbraten|vegetarian loaf|veggie loaf|gemusebraten|pastete|terrine|leberkas laib|fleischlaib',
		'loaf',
		'patty',
		'brown',
		['herb', 'swoosh'],
		true
	),
	d(
		'shepherds-pie',
		"Shepherd's pie",
		'shepherd pie|shepherds pie|cottage pie|hackfleisch auflauf|hackauflauf|kartoffel hackfleisch auflauf|kartoffelauflauf mit hack|fisherman pie|fish pie|fischauflauf|kartoffelauflauf|mince pie|moussaka|mousaka|musaka|pastitsio|pastizio|greek casserole|griechischer auflauf|lamb pie|lammauflauf|bolognese auflauf|bolognese bake|hackfleischauflauf|potato mince bake|hack kartoffel auflauf|hackfleisch kartoffel auflauf|kartoffel hack',
		'tray',
		'mash',
		'cream',
		['mince', 'cheese'],
		true
	),
	d(
		'stroganoff',
		'Stroganoff',
		'stroganoff|beef stroganoff|mushroom stroganoff|pilz stroganoff|rindsgeschnetzeltes|rahmgeschnetzeltes|pilzgeschnetzeltes|champignon rahm|rahmchampignons|rahmpilze|creamy mushrooms|champignons in rahm|pilzpfanne|pilze in rahmsauce|hahnchen in rahmsauce|chicken in cream sauce|cream sauce chicken|rahmhahnchen|rahmsauce|pepper sauce|pfeffersauce|rahmschnitzel|jagersauce',
		'plate',
		'creamy',
		'brown',
		['mushroom', 'slice', 'herb'],
		true,
		'rice',
		'cream'
	),

	// ---- German classics ----
	d(
		'bratkartoffeln',
		'Bratkartoffeln',
		'bratkartoffeln|bratkartoffel|fried potatoes|pan fried potatoes|hash browns|hash brown|rosti|rösti|kartoffelpfanne|pfannenkartoffeln|potato hash|bauernfruhstuck pfanne|kartoffeln gebraten|country potatoes|skillet potatoes|breakfast potatoes|kartoffel pfanne|sauteed potatoes|rosmarinkartoffeln pfanne|hash|kartoffelrosti|kartoffel rosti|potato rosti|rosti mit|kartoffelgulasch|bratkartoffeln mit',
		'pan',
		'potatoes',
		'wheat',
		['onion', 'bacon', 'herb'],
		true
	),
	d(
		'kartoffelpuffer',
		'Kartoffelpuffer',
		'kartoffelpuffer|reibekuchen|kartoffelpfannkuchen|potato pancakes|latkes|latke|reiberdatschi|kartoffelplatzchen|grumbeerpannekuche|kartoffel puffer|potato pancake|rosti puffer|reibeplatzchen|erdapfelpuffer|raggmunk|boxty|tattie scones|potato cakes|kartoffelkuchen herzhaft|kartoffeltaler|kartoffelrosti puffer',
		'plate',
		'patty',
		'pumpkin',
		['apple', 'dollop', 'herb'],
		true
	),
	d(
		'kasespatzle',
		'Käsespätzle',
		'kasespatzle|spatzle|kasespatzeln|knopfle|spatzle mit|kassespatzle|kaesespaetzle|allgauer kasespatzle|spatzlepfanne|spatzle pfanne|spaetzle|kaas spatzle|cheese spaetzle|schupfnudeln|schupfnudel|krautfleckerl|kasnocken|kaesnocken|krautspatzle|kartoffel nudeln|bubespitzle|fingernudeln|gnocchi pfanne kase|kaese spatzle|spatzle auflauf|spaetzle auflauf',
		'pan',
		'gnocchi',
		'wheat',
		['cheese', 'onion', 'herb'],
		true
	),
	d(
		'rouladen',
		'Rinderrouladen',
		'rouladen|rinderrouladen|roulade|beef roulade|beef rolls|rinderroulade|kohlroulade|kohlrouladen|wirsingrouladen|schweinerouladen|hahnchenrouladen|putenrouladen|involtini|braciole|gefullte rouladen|rouladen mit|rinderrouladen mit|beef olives|saltimbocca|cordon|rolled beef|rolled pork|rollbraten|rolle|schinkenrollen',
		'plate',
		'roast',
		'brown',
		['slice', 'onion', 'cabbage'],
		true,
		'potatoes',
		'wheat'
	),
	d(
		'konigsberger-klopse',
		'Königsberger Klopse',
		'konigsberger klopse|konigsberger|klopse|klops|klopse in kapernsosse|kapernsosse|kochklopse|saure klopse|fleischklopse mit kapern|meatballs in caper sauce|caper sauce|konigsberger klopse mit|kapern klopse|kapernklopse|weisse sosse|white sauce meatballs|weisse klopse|fleischklopse in weisser sosse|hackfleischklopse|hackklopse|kochfleisch|kochfleisch klopse|knodel in sosse|hackfleischknodel|hackknodel',
		'plate',
		'patty',
		'cream',
		['dot', 'herb'],
		true,
		'potatoes',
		'wheat'
	),
	d(
		'maultaschen',
		'Maultaschen',
		'maultaschen|maultasche|schwabische maultaschen|geschmelzte maultaschen|maultaschen suppe|maultaschensuppe|maultaschen pfanne|maultaschen mit|maultaschen in brühe|maultaschen in bruhe|wabische maultaschen|herrgottsbescheisserle|pfannkuchen suppe|flädlesuppe|fladlesuppe|flädle|fladle|frittatensuppe|griessnockerl|grießnockerl|leberknodel|leberknodelsuppe|leberknodel suppe|markklosschen|markklosse|leberspatzle|leberspatzle suppe|ravioli suppe|tortellini suppe|tortellini brühe|tortellini bruhe|tortellini in brodo|brodo|cappelletti in brodo|agnolotti|pelmeni suppe|tortelloni|tortelloni mit',
		'pan',
		'dumpling',
		'cream',
		['onion', 'egg', 'herb'],
		true
	),
	d(
		'kaiserschmarrn',
		'Kaiserschmarrn',
		'kaiserschmarrn|schmarrn|kaiserschmarren|scheiterhaufen|apfelschmarrn|topfenschmarrn|griessschmarrn|grießschmarrn|semmelschmarrn|kirschmichel|plumpudding|germknodel|germknödel|mohnnudeln|powidl|powidltascherl|zwetschgenknodel|marillenknodel|topfenknodel|quarkkeulchen|quarkkeulchen sachsische|quarkkuchle|quarkbällchen|quarkbaellchen|quarkbällchen|berliner|krapfen|pfannkuchen zerrissen|torn pancake|shredded pancake|emperors pancake|emperor pancake|fluffy torn pancake|kaiserschmarrn mit|sweet torn pancake|kaiser schmarrn|kaiserschmarrn rezept|dessert pancake torn|schmarren|apfelschmarren|schmarren mit|rhabarber schmarrn|rhabarberschmarrn|beeren schmarrn|beerenschmarrn|zwetschgen schmarrn|zwetschgenschmarrn|nuss schmarrn|nussschmarrn|griess schmarrn|griessschmarren|griesschmarrn',
		'plate',
		'pancake',
		'pumpkin',
		['berry', 'sprinkle', 'apple'],
		true
	),
	d(
		'currywurst',
		'Currywurst',
		'currywurst|curry wurst|currywurst mit|currywurst pommes|currywurst teller|currywurst pfanne|currywurst mit pommes|currywurst rezept|berliner currywurst|ruhrpott currywurst|currywurst sauce|currywurstsosse|currywurst sosse|curry ketchup|currysosse|currysauce|currywurst mit brötchen|currywurst mit brotchen|currywurst pommes rot weiss|pommes rot weiss|rot weiss|pommes schranke|pommes spezial|pommes mit|pommes frites|pommes|fries|french fries|chips|frites|steakhouse fries|waffle fries|curly fries|wedges fries|chili cheese fries|loaded fries|poutine|kartoffelstabchen|kartoffelspalten pommes|ofenpommes|oven fries|baked fries|susskartoffelpommes|sweet potato fries|susskartoffel pommes|airfryer fries|airfryer pommes|heissluftfritteuse pommes|pommes aus dem ofen|pommes selber machen|selbstgemachte pommes|homemade fries|home made fries|hausgemachte pommes',
		'plate',
		'fries',
		'wheat',
		['sausage', 'swoosh', 'sprinkle'],
		true
	),
	d(
		'leberkase',
		'Leberkäse',
		'leberkase|fleischkase|leberkasesemmel|leberkas|leberkassemmel|fleischkäse|leberkäse|leberkase semmel|leberkase mit|leberkase mit spiegelei|leberkas mit|leberkasbrotchen|leberkasbrötchen|fleischkasebrotchen|fleischkasesemmel|fleischkase semmel|fleischkase brotchen|fleischkase mit|fleischkase mit spiegelei|bayerischer leberkase|bavarian meatloaf|bayerischer fleischkase|bavarian leberkase|leberkaese|fleischkaese|leberkaese semmel|leberkaese mit|fleischkaese semmel|fleischkaese mit',
		'bun',
		'patty',
		'pink',
		['swoosh', 'onion'],
		true
	),
	d(
		'sauerbraten',
		'Sauerbraten',
		'sauerbraten|rheinischer sauerbraten|sauerbraten mit|sauerbraten mit klosen|sauerbraten mit knodeln|sauerbraten mit rotkohl|sauerbraten mit rotkraut|sauerbraten mit kartoffelklosen|sauerbraten rezept|marinated beef|marinierter rinderbraten|marinierter braten|beef marinated|sauer braten|sauerbratenrezept|pferdesauerbraten|hirschbraten|rehbraten|wildbraten|wild|wildgulasch|hirschgulasch|rehgulasch|wildschwein|wildschweinbraten|venison|venison roast|venison stew|game stew|game roast|reh|hirsch|hase|hasenbraten|kaninchen|rabbit|kaninchenbraten|rabbit stew|kaninchen in senfsosse|rabbit in mustard sauce|taube|pigeon|ente|duck|entenbrust|duck breast|gans|goose|gänsebraten|gansebraten|martinsgans|weihnachtsgans|weihnachtsente|knusprige ente|crispy duck|peking duck|pekingente|peking ente',
		'plate',
		'roast',
		'dark',
		['slice', 'carrot', 'cabbage'],
		true,
		'dumpling',
		'cream'
	),
	d(
		'knodel',
		'Knödel',
		'knodel|semmelknodel|kartoffelknodel|kloss|klosse|kloße|knodel mit|serviettenknodel|bohmische knodel|bohmische|speckknodel|spinatknodel|kasknodel|tiroler knodel|pilzrahm knodel|knodel mit pilzen|knodel mit pilzrahm|semmelknodel mit pilzen|semmelknodel mit pilzrahm|semmelknodel mit schwammerlsauce|schwammerlsauce|schwammerl|schwammerlgulasch|pilzgulasch|dumplings with mushroom sauce|bread dumplings|potato dumplings|bread dumpling|potato dumpling|thuringer klosse|thüringer klöße|thuringer kloesse|griesklosschen|griessklosschen|gnudi|canederli|strangolapreti|knoedel|semmelknoedel|kartoffelknoedel|serviettenknoedel|boehmische knoedel|speckknoedel|spinatknoedel|kasknoedel|tiroler knoedel',
		'plate',
		'dumpling',
		'cream',
		['mushroom', 'herb'],
		true
	),
	d(
		'pellkartoffeln',
		'Pellkartoffeln mit Quark',
		'pellkartoffeln|quark kartoffeln|kartoffeln mit quark|pellkartoffeln mit quark|salzkartoffeln|boiled potatoes|new potatoes|kartoffeln mit kräuterquark|kartoffeln mit krauterquark|pellkartoffeln mit krauterquark|neue kartoffeln|jacket boiled potatoes|steamed potatoes|gekochte kartoffeln|kartoffeln|potatoes|potato|petersilienkartoffeln|parsley potatoes|buttered potatoes|butterkartoffeln|salzkartoffel|pellkartoffel|kartoffel',
		'plate',
		'potatoes',
		'wheat',
		['dollop', 'herb', 'onion'],
		true
	),

	// ---- salads ----
	d(
		'greek-salad',
		'Greek salad',
		'greek salad|griechischer salat|bauernsalat|horiatiki|choriatiki|greek|griechisch|griechischer bauernsalat|feta salad|fetasalat|schafskase salat|shepherd salad|shepherds salad|hirtensalat|schopska|shopska|shopska salad|tomato cucumber feta|tomaten gurken feta|salata horiatiki|mediterranean salad|mediterraner salat|mediterraner|mediterranean|mittelmeer salat|mittelmeersalat|turkish salad|turkischer salat|hirtensalat turkisch|coban salatasi|coban|cacik|ezme|gavurdagi|gavurdagi salatasi|piyaz|kisir|fattoush|fattoush salad|fattouch',
		'salad',
		'greens',
		'leaf',
		['cucumber', 'tomato', 'olive', 'cube'],
		false
	),
	d(
		'caesar-salad',
		'Caesar salad',
		'caesar salad|caesar|casarsalat|cesar salad|caesarsalat|chicken caesar|hahnchen caesar|caesar dressing|casar salat|cesar|romaine salad|romanasalat|römersalat|romersalat|chicken caesar salad|caesar wrap|cesar salat',
		'salad',
		'greens',
		'leaf',
		['crouton', 'cheese', 'chicken'],
		false
	),
	d(
		'caprese',
		'Caprese',
		'caprese|tomate mozzarella|tomato mozzarella|insalata caprese|caprese salad|capresesalat|mozzarella tomaten|tomaten mozzarella|mozzarella salad|mozzarella salat|burrata|burrata salad|burrata salat|tomato burrata|tomaten burrata|caprese toast|caprese sandwich|caprese skewers|caprese spiesse|caprese pasta|caprese nudeln|caprese tart|caprese tarte|caprese auflauf|caprese bake|caprese chicken|caprese hahnchen|hahnchen caprese|chicken caprese|insalata|tomato basil mozzarella|tomaten basilikum mozzarella',
		'salad',
		'none',
		'leaf',
		['mozzarella', 'tomato', 'basil'],
		false
	),
	d(
		'potato-salad',
		'Potato salad',
		'potato salad|kartoffelsalat|warm potato salad|warmer kartoffelsalat|schwabischer kartoffelsalat|kartoffel salat|erdapfelsalat|german potato salad|deutscher kartoffelsalat|kartoffelsalat mit|kartoffelsalat mit mayo|kartoffelsalat mit brühe|kartoffelsalat mit bruhe|kartoffelsalat mit essig und ol|kartoffelsalat mit speck|kartoffelsalat mit gurke|kartoffelsalat mit ei|kartoffelsalat mit würstchen|kartoffelsalat mit wurstchen|kartoffelsalat und wurstchen|kartoffelsalat und wurst|kartoffelsalat und frikadellen|kartoffelsalat mit frikadellen|kartoffelsalat mit schnitzel|kartoffelsalat und schnitzel|kartoffelsalat mit fisch|kartoffelsalat und fisch|kartoffelsalat mit hering|kartoffelsalat und hering',
		'bowl',
		'potatoes',
		'cream',
		['herb', 'egg', 'onion'],
		false
	),
	d(
		'couscous-salad',
		'Couscous salad',
		'couscous|couscous salad|couscoussalat|bulgur salad|bulgursalat|bulgur|grain salad|getreidesalat|cous cous|kuskus|kuskussalat|cous cous salat|cous cous salad|israeli couscous|perlcouscous|pearl couscous|farro salad|farro|dinkelsalat|dinkel salat|dinkel|gerstensalat|barley salad|graupen|graupensalat|rollgerste|hirsesalat|millet salad|hirse|millet|freekeh|freekeh salad|wheat berry salad|weizensalat|wheat salad|weizenbulgur|spelt salad|spelt',
		'salad',
		'grain',
		'wheat',
		['tomato', 'cucumber', 'herb', 'chickpea'],
		false
	),
	d(
		'quinoa-salad',
		'Quinoa bowl',
		'quinoa|quinoa salad|quinoasalat|quinoa bowl|quinoa schussel|quinoa pfanne|quinoa pan|quinoa mit|quinoa und|quinoa and|amaranth|amaranth salad|buchweizen|buckwheat|buchweizensalat|buckwheat salad|kasha|kascha|buchweizen pfanne|buchweizen bowl|buckwheat bowl|chia quinoa|teff|sorghum|wild rice|wildreis|wildreis salat|wild rice salad|black rice|schwarzer reis|red rice|roter reis|brown rice|naturreis|vollkornreis|brown rice bowl|naturreis bowl|vollkornreis bowl|brown rice salad|naturreis salat|vollkornreis salat',
		'bowl',
		'grain',
		'pea',
		['avocado', 'bean', 'corn', 'pepper'],
		false
	),
	d(
		'tabbouleh',
		'Tabbouleh',
		'tabbouleh|tabouleh|taboule|tabouli|tabule|tabbouli|parsley salad|petersiliensalat|petersilien salat|libanesischer salat|lebanese salad|lebanese|libanesisch|syrian salad|syrischer salat',
		'salad',
		'grain',
		'leaf',
		['herb', 'tomato', 'lemon', 'cucumber'],
		false
	),
	d(
		'coleslaw',
		'Coleslaw',
		'coleslaw|krautsalat|cole slaw|weisskohlsalat|rotkohlsalat|kohlsalat|slaw|red cabbage salad|cabbage salad|kraut salat|weisskraut salat|weisskrautsalat|rotkraut salat|rotkrautsalat|sauerkraut salat|sauerkrautsalat|karottensalat|carrot salad|moehrensalat|mohrensalat|rohkost|rohkostsalat|raw vegetable salad|gemuse rohkost|rohkostteller|rohkost teller|crudites|crudite|rohkost mit dip|crudites with dip|veggie sticks|gemuse sticks|gemusesticks|gemuse stick|stick|sticks',
		'small',
		'greens',
		'milk',
		['cabbage', 'carrot', 'dollop'],
		false
	),
	d(
		'nicoise',
		'Salade niçoise',
		'nicoise|niçoise|salade nicoise|salad nicoise|nicoise salad|tuna salad|thunfischsalat|thunfisch salat|thunfisch|tuna|tuna nicoise|thunfisch nicoise|tonno|insalata di tonno|tuna bowl|thunfisch bowl|thunfisch pasta|tuna pasta|tuna melt toast|thunfisch toast|tuna toast|tuna sandwich|thunfisch sandwich|thunfisch wrap|tuna wrap|tuna rice|thunfisch reis|tuna casserole|thunfisch auflauf|tuna bake|thunfisch bake|tuna patties|thunfisch frikadellen|tuna burger|thunfisch burger|tuna steak salad|thunfischsteak salat|seared tuna|angebratener thunfisch|tuna tataki|thunfisch tataki|tataki|tuna poke|thunfisch poke|poke tuna|poke thunfisch|ahi tuna|ahi|ahi poke|ahi bowl',
		'salad',
		'greens',
		'leaf',
		['egg', 'olive', 'tomato', 'fillet'],
		false
	),
	d(
		'waldorf',
		'Waldorf salad',
		'waldorf|waldorf salad|waldorfsalat|waldorf salat|apple salad|apfelsalat|apfel salat|celery salad|selleriesalat|sellerie salat|apfel sellerie salat|apple celery salad|nut salad|nusssalat|nuss salat|walnuss salat|walnut salad|walnusssalat|walnut apple salad|walnuss apfel salat|apfel walnuss salat|apple walnut salad|chicken waldorf|hahnchen waldorf|waldorf chicken|waldorf hahnchen|waldorf wrap|waldorf sandwich|waldorf toast|waldorf bowl|waldorf schussel',
		'salad',
		'greens',
		'leaf',
		['apple', 'nut', 'leaf'],
		false
	),
	d(
		'green-salad',
		'Green salad',
		'green salad|salat|salad|gruner salat|mixed salad|side salad|gemischter salat|beilagensalat|blattsalat|leaf salad|garden salad|house salad|salatteller|salat teller|salad plate|summer salad|sommersalat|sommer salat|winter salad|wintersalat|winter salat|autumn salad|herbstsalat|herbst salat|spring salad|fruhlingssalat|fruhlings salat|salat mit|salad with|salat bowl|salad bowl|salat schussel|salad dressing|dressing|salatdressing|vinaigrette|salat sosse|salatsosse|rucola|rucola salat|rocket|rocket salad|arugula|arugula salad|feldsalat|lamb s lettuce|lambs lettuce|eisbergsalat|iceberg|iceberg salad|romaine|kopfsalat|butterhead|bataviasalat|lollo rosso|endiviensalat|endive|endive salad|chicoree|chicory|radicchio|frisee|spinatsalat|spinach salad|babyspinat salat|baby spinach salad|spinach and strawberry salad|erdbeersalat|strawberry salad|melon salad|melonensalat|wassermelonensalat|watermelon salad|watermelon feta|wassermelone feta|wassermelone',
		'salad',
		'greens',
		'leaf',
		['tomato', 'cucumber'],
		false
	),
	d(
		'tomato-salad',
		'Tomato salad',
		'tomato salad|tomatensalat|tomaten salat|tomato onion salad|tomaten zwiebel salat|tomato basil salad|tomaten basilikum salat|tomato and onion|tomaten und zwiebeln|tomaten mit zwiebeln|tomaten mit basilikum|tomatoes with basil|tomatoes with onion|tomatoes and onion|tomatoes and basil|tomatoes',
		'salad',
		'greens',
		'leaf',
		['tomato', 'onion', 'herb'],
		false
	),
	d(
		'cucumber-salad',
		'Cucumber salad',
		'cucumber salad|gurkensalat|gurken salat|cucumber yoghurt|cucumber yogurt|gurken joghurt|gurke joghurt|cucumber dill|gurken dill|dill gurken|dillgurken|saure gurken|pickles|pickled cucumber|eingelegte gurken|pickled vegetables|eingelegtes gemuse|eingelegt|pickled|essiggurken|cornichons|gewurzgurken|senfgurken|sauer eingelegt|asian cucumber salad|asiatischer gurkensalat|sesam gurkensalat|sesame cucumber salad|smashed cucumber|smashed cucumber salad|zerschlagene gurken|gurken',
		'salad',
		'greens',
		'leaf',
		['cucumber', 'dollop', 'herb'],
		false
	),
	d(
		'chicken-salad',
		'Chicken salad',
		'chicken salad|hahnchensalat|huhnchensalat|hahnchen salat|huhnchen salat|salat mit hahnchen|salat mit huhnchen|salat mit hahnchenbrust|salad with chicken|salad with grilled chicken|grilled chicken salad|gegrilltes hahnchen salat|geflugelsalat|geflügelsalat|poultry salad|turkey salad|putensalat|puten salat|salat mit putenstreifen|salat mit putenbrust|chicken avocado salad|hahnchen avocado salat|avocado chicken salad|avocado hahnchen salat|chicken bacon salad|hahnchen speck salat|chicken and bacon salad|hahnchen und speck salat|ranch chicken salad|ranch hahnchen salat|ranch salad|ranch salat|cobb salad|cobb|cobb salat|cobbsalat|taco salad|taco salat|tacosalat|mexican salad|mexikanischer salat|mexikanischer bohnensalat|mexican bean salad|bean and corn salad|bohnen mais salat|mais salat|corn salad|maissalat|corn and tomato salad|mais tomaten salat|mais und tomaten salat',
		'salad',
		'greens',
		'leaf',
		['chicken', 'tomato', 'avocado'],
		false
	),
	d(
		'lentil-salad',
		'Lentil salad',
		'lentil salad|linsensalat|linsen salat|warm lentil salad|warmer linsensalat|french lentil salad|puy lentil salad|beluga linsensalat|beluga lentil salad|beluga lentils|beluga linsen|puy lentils|puy linsen|belugalinsen|brown lentil salad|braune linsen salat|green lentil salad|grune linsen salat|lentil bowl|linsen bowl|linsen schussel|lentil and feta salad|linsen feta salat|lentil and beet salad|linsen rote bete salat|lentil beet salad|lentil carrot salad|linsen karotten salat|lentils',
		'bowl',
		'grain',
		'brown',
		['lentil', 'carrot', 'herb', 'cube'],
		false
	),
	d(
		'chickpea-salad',
		'Chickpea salad',
		'chickpea salad|kichererbsensalat|kichererbsen salat|chickpea and cucumber salad|kichererbsen gurken salat|chickpea tomato salad|kichererbsen tomaten salat|chickpea feta salad|kichererbsen feta salat|chickpea avocado salad|kichererbsen avocado salat|smashed chickpea salad|chickpea salad sandwich|chickpea tuna|kichererbsen thunfisch|mock tuna|mock tuna salad|vegan tuna|vegan tuna salad|bean salad|bohnensalat|bohnen salat|three bean salad|drei bohnen salat|white bean salad|weisse bohnen salat|weisser bohnensalat|kidney bean salad|kidneybohnen salat|black bean salad|schwarze bohnen salat|black bean and corn salad|schwarze bohnen mais salat|green bean salad|grune bohnen salat|grune bohnen|green beans|bohnen|beans',
		'salad',
		'greens',
		'leaf',
		['chickpea', 'cucumber', 'tomato'],
		false
	),
	d(
		'beetroot-salad',
		'Beetroot salad',
		'beetroot salad|rote bete salat|rote beete salat|beet salad|beetroot feta|rote bete feta|beetroot goat cheese|rote bete ziegenkase|beetroot orange|rote bete orange|beetroot and apple|rote bete apfel|rote bete meerrettich|beetroot horseradish|beetroot carpaccio|rote bete carpaccio|roasted beetroot|ofen rote bete|gebackene rote bete|baked beetroot|beetroot hummus|rote bete hummus|beetroot dip|rote bete dip|beetroot|rote bete|rote beete|beet|beets',
		'salad',
		'greens',
		'pink',
		['beet', 'cube', 'nut'],
		false
	),
	d(
		'buddha-bowl',
		'Buddha bowl',
		'buddha bowl|power bowl|grain bowl|bowl|schussel|nourish bowl|veggie bowl|gemuse bowl|gemuseschussel|vegetable bowl|protein bowl|energy bowl|glow bowl|sunshine bowl|rainbow bowl|regenbogen bowl|harvest bowl|herbst bowl|autumn bowl|winter bowl|spring bowl|summer bowl|sommer bowl|mediterranean bowl|mediterrane bowl|greek bowl|griechische bowl|asian bowl|asia bowl|asiatische bowl|korean bowl|koreanische bowl|japanese bowl|japanische bowl|mexican bowl style|southwest bowl|chicken bowl|hahnchen bowl|beef bowl|rind bowl|rindfleisch bowl|pork bowl|schwein bowl|schweinefleisch bowl|tofu bowl|falafel bowl style|salmon bowl style|shrimp bowl|garnelen bowl|prawn bowl|egg bowl|ei bowl|eier bowl|sweet potato bowl|susskartoffel bowl|roasted veg bowl|ofengemuse bowl|chickpea bowl|kichererbsen bowl|lentil bowl style|bean bowl|bohnen bowl|avocado bowl|breakfast grain bowl|fruhstucks bowl',
		'bowl',
		'grain',
		'cream',
		['avocado', 'chickpea', 'carrot', 'leaf'],
		false
	),

	// ---- vegetables ----
	d(
		'veg-traybake',
		'Roasted vegetable traybake',
		'traybake|tray bake|roasted vegetables|ofengemuse|roast vegetables|gebackenes gemuse|sheet pan|sheet pan dinner|ofengemuse mit|gemuse aus dem ofen|roasted veg|roast veg|vegetable traybake|veggie traybake|gemuse traybake|one tray|one pan|one pan dinner|ofenblech|blechgericht|blech gericht|gemuse vom blech|gemusepfanne ofen|ofen gemusepfanne|oven roasted vegetables|ofengeroestetes gemuse|ofengeroestet|ofengeröstet|roasted|geröstet|gerostet|geroestet|chicken traybake|hahnchen traybake|sausage traybake|wurst traybake|salmon traybake|lachs traybake|halloumi traybake|halloumi tray|halloumi bake|halloumi und kichererbsen',
		'tray',
		'traybake',
		'pumpkin',
		['carrot', 'pepper', 'zucchini', 'onion'],
		true
	),
	d(
		'gratin',
		'Gratin',
		'gratin|kartoffelgratin|potato gratin|dauphinoise|auflauf|gemuseauflauf|cauliflower cheese|gemuse gratin|casserole|bake|baked|ofengericht|oven bake|ofen auflauf|ofenauflauf|vegetable bake|gemuse bake|veggie bake|kartoffel gratin|kartoffelauflauf kase|kasekartoffeln|cheesy potatoes|kase kartoffeln|scalloped potatoes|potatoes au gratin|gratinated|gratiniert|uberbacken|uberbackene|ueberbacken|überbacken|überbackene|zucchini gratin|zucchini auflauf|zucchinigratin|blumenkohl gratin|blumenkohlauflauf|blumenkohl auflauf|brokkoli gratin|brokkoli auflauf|brokkoliauflauf|broccoli gratin|broccoli bake|broccoli cheese bake|spinat gratin|spinat auflauf|spinatauflauf|spinach gratin|spinach bake|lauch gratin|lauch auflauf|lauchauflauf|leek gratin|leek bake|chicory gratin|chicoree gratin|chicoree auflauf|chicoree schinken auflauf|fenchel gratin|fenchel auflauf|fennel gratin|fennel bake|sweet potato gratin|susskartoffel gratin|susskartoffel auflauf|susskartoffelauflauf|pumpkin gratin|kurbis gratin|kurbis auflauf|kurbisauflauf',
		'tray',
		'gratin',
		'cream',
		['potato', 'cheese', 'herb'],
		true
	),
	d(
		'stuffed-peppers',
		'Stuffed peppers',
		'stuffed peppers|stuffed pepper|gefullte paprika|paprika gefullt|stuffed courgette|stuffed zucchini|gefullte zucchini|stuffed aubergine|gefullte aubergine|stuffed tomatoes|gefullte tomaten|stuffed mushrooms|gefullte champignons|stuffed vegetables|gefulltes gemuse|stuffed squash|gefullter kurbis|stuffed pumpkin|stuffed potatoes|gefullte kartoffeln|gefullte ofenkartoffeln|stuffed sweet potato|gefullte susskartoffel|stuffed cabbage|gefullter kohl|gefullte paprikaschoten|paprikaschoten|dolma|dolmades|sarma|gefullte weinblatter|stuffed vine leaves|stuffed onions|gefullte zwiebeln|gefulltes|stuffed|gefullt|zucchini boats|zucchinischiffchen|aubergine boats|auberginenschiffchen|pepper boats|paprikaschiffchen|imam bayildi|karniyarik|stuffed eggplant',
		'tray',
		'traybake',
		'terra',
		['pepper', 'cheese', 'herb'],
		true
	),
	d(
		'ratatouille',
		'Ratatouille',
		'ratatouille|ratatouille mit|gemuse ragout|vegetable ragout|gemuseragout|veggie ragout|ragout|vegetable stew|gemuse stew|veggie stew|mediterranean vegetables|mediterranes gemuse|mediterrane gemusepfanne|mediterranean vegetable stew|mediterranes gemuse eintopf|caponata|peperonata|briam|tian|confit byaldi|byaldi|turlu|turlu turlu|tourlou|tourlou tourlou|gemuse tian|vegetable tian|gemusetopf|gemuse topf|gemusegulasch|gemuse gulasch|vegetable goulash|veggie goulash|vegetarian goulash|vegetarisches gulasch|pilzgulasch pot|paprikagemuse|paprika gemuse|paprika ragout|paprika eintopf|paprika topf|paprikatopf|zucchini topf|zucchinitopf|zucchini ragout|zucchiniragout|zucchini eintopf|zucchinieintopf|aubergine topf|auberginentopf|aubergine ragout|auberginenragout|aubergine eintopf|auberginen eintopf|auberginen',
		'pot',
		'thick',
		'terra',
		['zucchini', 'aubergine', 'pepper', 'tomato'],
		true
	),
	d(
		'stir-fried-greens',
		'Garlicky greens',
		'stir fried greens|garlic greens|pak choi|bok choy|sauteed spinach|creamed spinach|rahmspinat|wilted greens|spinach pan|spinatpfanne|spinat pfanne|spinat|blattspinat|rahm spinat|spinat mit|spinach with|sauteed greens|gebratener spinat|gebratener pak choi|gebratenes gemuse|stir fried pak choi|stir fried bok choy|stir fried spinach|stir fried kale|stir fried cabbage|stir fried broccoli|stir fried green beans|stir fried asparagus|stir fried vegetables garlic|garlic broccoli|knoblauch brokkoli|brokkoli pfanne|brokkolipfanne|broccoli pan|broccoli stir fry|brokkoli stir fry|gebratener brokkoli|gebratene brokkoli|gebratener kohl|gebratener weisskohl|gebratene weisskohl|kohlpfanne|kohl pfanne|cabbage pan|cabbage stir fry|kohl stir fry|kohlgemuse|kohl gemuse|wirsing|wirsingpfanne|wirsing pfanne|wirsinggemuse|wirsing gemuse|grunkohl|grunkohl pfanne|grunkohlpfanne|kale|kale pan|kale stir fry|grunkohl stir fry|palmkohl|palmkohl pfanne|palmkohlpfanne|spitzkohl|spitzkohl pfanne|spitzkohlpfanne|spitzkohl gemuse|spitzkohlgemuse|chinakohl|chinakohl pfanne|chinakohlpfanne|chinakohl gemuse|chinakohlgemuse|mangold|mangold pfanne|mangoldpfanne|mangold gemuse|mangoldgemuse|swiss chard|chard|chard pan|chard stir fry|mangold stir fry',
		'pan',
		'greens',
		'leaf',
		['garlic', 'spinach'],
		true
	),
	d(
		'baked-potato',
		'Baked potato',
		'baked potato|jacket potato|ofenkartoffel|ofenkartoffeln|backkartoffel|backkartoffeln|folienkartoffel|folienkartoffeln|loaded potato|baked potatoes|jacket potatoes|loaded baked potato|ofenkartoffel mit|ofenkartoffeln mit|folienkartoffel mit|folienkartoffeln mit|backkartoffel mit|backkartoffeln mit|baked potato with|jacket potato with|stuffed baked potato|twice baked potato|zweimal gebackene kartoffel|baked sweet potato|ofen susskartoffel|susskartoffel ofen|gebackene susskartoffel|gebackene kartoffel|gebackene kartoffeln|kartoffel aus dem ofen|kartoffeln aus dem ofen|ofen kartoffel|ofen kartoffeln|potato with|kartoffel mit',
		'plate',
		'potatoes',
		'wheat',
		['dollop', 'cheese', 'spring-onion', 'bacon'],
		true
	),
	d(
		'roast-potatoes',
		'Roast potatoes',
		'roast potatoes|roasted potatoes|rosmarinkartoffeln|wedges|potato wedges|kartoffelspalten|ofenkartoffeln rosmarin|rosemary potatoes|rosemary roast potatoes|gebackene kartoffeln|oven potatoes|oven roasted potatoes|ofengeroestete kartoffeln|ofengeröstete kartoffeln|ofen kartoffeln rosmarin|kartoffeln rosmarin|kartoffeln aus dem ofen mit rosmarin|rosmarin kartoffeln|rosmarinkartoffel|kartoffelecken|kartoffel ecken|kartoffel wedges|kartoffelwedges|country wedges|steakhouse wedges|spicy wedges|wuerzige kartoffelspalten|wurzige kartoffelspalten|gewurzkartoffeln|gewuerzkartoffeln|gewurz kartoffeln|gewuerz kartoffeln|hasselback|hasselback potatoes|hasselback kartoffeln|hasselbackkartoffeln|schwedische kartoffeln|swedish potatoes|fächerkartoffeln|facherkartoffeln|fächer kartoffeln|facher kartoffeln|accordion potatoes|akkordeon kartoffeln|akkordeonkartoffeln|duchess potatoes|herzogin kartoffeln|herzoginkartoffeln|pommes duchesse|duchesse',
		'tray',
		'potatoes',
		'wheat',
		['herb', 'garlic', 'wedge'],
		true
	),
	d(
		'fritters',
		'Vegetable fritters',
		'fritters|fritter|zucchinipuffer|gemusepuffer|corn fritters|zucchini fritters|gemuse puffer|zucchini puffer|veggie fritters|vegetable fritters|veggie puffer|gemuseplatzchen|gemuse platzchen|gemusebratlinge|gemuse bratlinge|bratlinge|bratling|veggie patties|veggie pattie|vegetable patties|vegetable pattie|gemusefrikadellen|gemuse frikadellen|gemuse laibchen|gemuselaibchen|laibchen|pakora|pakoras|bhaji|bhajis|onion bhaji|zwiebel bhaji|mucver|ficelle|okonomiyaki|chickpea fritters|kichererbsen puffer|kichererbsenpuffer|linsenpuffer|lentil fritters|spinat puffer|spinatpuffer|spinach fritters|karottenpuffer|karotten puffer|carrot fritters|susskartoffelpuffer|susskartoffel puffer|sweet potato fritters|kurbispuffer|kurbis puffer|pumpkin fritters|brokkolipuffer|brokkoli puffer|broccoli fritters|blumenkohlpuffer|blumenkohl puffer|cauliflower fritters|kasepuffer|kase puffer|cheese fritters|halloumi fritters|halloumi puffer',
		'plate',
		'patty',
		'leaf',
		['zucchini', 'dollop', 'herb'],
		true
	),

	// ---- bakes and desserts ----
	d(
		'crumble',
		'Crumble',
		'crumble|apple crumble|apfelcrumble|plum crumble|pflaumencrumble|rhubarb crumble|rhabarbercrumble|berry crumble|beerencrumble|fruit crumble|obstcrumble|streusel|apfelstreusel|streuselkuchen|apple streusel|pear crumble|birnencrumble|birnen crumble|apfel crumble|pflaumen crumble|rhabarber crumble|beeren crumble|obst crumble|cobbler|peach cobbler|pfirsich cobbler|apple cobbler|apfel cobbler|berry cobbler|beeren cobbler|fruit cobbler|obst cobbler|betty|apple brown betty|brown betty|bratapfel|gebackener apfel|baked apple|baked apples|bratapfel mit|gebackene apfel',
		'slice',
		'crumble',
		'wheat',
		['berry', 'apple'],
		true
	),
	d(
		'apple-pie',
		'Apple pie',
		'apple pie|apfelkuchen|apfeltarte|tarte tatin|apple tart|apfel tarte|apfelkuchen mit|apfelkuchen vom blech|apfelkuchen blech|versunkener apfelkuchen|sunken apple cake|apple cake|gedeckter apfelkuchen|covered apple cake|apfelstrudel kuchen|apfeltorte|apfel torte|apple torte|apple tart tatin|tatin|tarte|tart|obstkuchen|obst kuchen|fruit tart|fruit pie|obsttarte|obst tarte|obsttorte|obst torte|fruit torte|cherry pie|kirschkuchen|kirsch kuchen|cherry cake|kirschtorte|kirsch torte|cherry torte|pflaumenkuchen|pflaumen kuchen|plum cake|zwetschgenkuchen|zwetschgen kuchen|zwetschgenkuchen vom blech|zwetschgendatschi|datschi|plum pie|plum tart|pflaumentarte|zwetschgentarte|pear tart|birnentarte|birnen tarte|pear pie|birnenkuchen|birnen kuchen|pear cake|berry pie|beerenkuchen|beeren kuchen|berry tart|beerentarte|beeren tarte|berry cake|beerentorte|beeren torte|berry torte|blueberry pie|heidelbeerkuchen|heidelbeer kuchen|blueberry cake|heidelbeertorte|heidelbeer torte|blueberry torte|blueberry tart|heidelbeertarte|heidelbeer tarte|rhubarb pie|rhabarberkuchen|rhabarber kuchen|rhubarb cake|rhabarbertorte|rhabarber torte|rhubarb torte|rhubarb tart|rhabarbertarte|rhabarber tarte|lemon tart|zitronentarte|zitronen tarte|lemon pie|zitronenkuchen|zitronen kuchen|lemon cake|zitronentorte|zitronen torte|lemon torte|lemon meringue pie|zitronen baiser kuchen|baiser|meringue|key lime pie|lime pie|limetten kuchen|pecan pie|pekannuss kuchen|walnut pie|walnuss kuchen|walnuss tarte|walnut tart|pumpkin pie|kurbiskuchen|kurbis kuchen|pumpkin cake|kurbistorte|kurbis torte|pumpkin torte|pumpkin tart|kurbistarte|kurbis tarte',
		'slice',
		'pie',
		'wheat',
		['apple', 'sprinkle'],
		true
	),
	d(
		'quiche',
		'Quiche',
		'quiche|quiche lorraine|zwiebelkuchen|herzhafte tarte|savoury tart|savory tart|speckkuchen|speck kuchen|bacon tart|spinach quiche|spinatquiche|spinat quiche|spinat tarte|spinach tart|gemusequiche|gemuse quiche|gemusetarte|gemuse tarte|vegetable quiche|vegetable tart|veggie quiche|veggie tart|lauchquiche|lauch quiche|lauchtarte|lauch tarte|leek quiche|leek tart|tomatentarte|tomaten tarte|tomato tart|tomato quiche|tomatenquiche|tomaten quiche|kasequiche|kase quiche|cheese quiche|cheese tart|kasetarte|kase tarte|ziegenkase tarte|goat cheese tart|ziegenkasetarte|ziegenkase quiche|goat cheese quiche|pilzquiche|pilz quiche|mushroom quiche|mushroom tart|pilztarte|pilz tarte|zucchinitarte|zucchini tarte|zucchini quiche|zucchiniquiche|zucchini tart|broccoli quiche|brokkoliquiche|brokkoli quiche|broccoli tart|brokkolitarte|brokkoli tarte|kurbisquiche|kurbis quiche|pumpkin quiche|kurbistarte herzhaft|pumpkin savoury tart|herzhafter kuchen|savoury cake|savory cake|herzhaftes gebäck|herzhaftes gebaeck|herzhaftes geback|savoury pastry|savory pastry|blatterteig|puff pastry|blatterteigtasche|blatterteigtaschen|puff pastry pockets|puff pastry turnovers|turnovers|turnover|blatterteig schnecken|blatterteigschnecken|puff pastry rolls|puff pastry swirls|pinwheels|pastry pinwheels|blatterteig rollen|blatterteigrollen|blatterteig stangen|blatterteigstangen|puff pastry sticks|cheese straws|kasestangen|kase stangen|pizza schnecken pastry|tarte au fromage|tarte aux oignons|tarte salee|salee|tarte aux legumes|tarte aux poireaux|tarte aux tomates|tarte au chevre|tarte aux champignons',
		'slice',
		'pie',
		'lemon',
		['cheese', 'herb'],
		true
	),
	d(
		'chicken-pie',
		'Chicken pie',
		'chicken pie|pot pie|meat pie|pie|fleischpastete|pastete|chicken pot pie|hahnchen pie|steak pie|steak and ale pie|beef pie|rind pie|pork pie|schweine pie|vegetable pie|gemuse pie|veggie pie|vegetable pot pie|gemuse pot pie|mushroom pie|pilz pie|pilzpastete|pilz pastete|mushroom pastete|spinach pie|spinat pie|spanakopita|spinat pastete|spinatpastete|cheese pie|kase pie|kasepastete|kase pastete|cheese and onion pie|kase zwiebel pie|potato pie|kartoffel pie|kartoffelpastete|kartoffel pastete|potato pastete|pasty|pasties|cornish pasty|cornish pasties|hand pie|hand pies|pocket pie|pocket pies|pie mit|pie with|pies|pastry|pastries|gebäck|geback|gebaeck|taschen|tasche|pocket|pockets',
		'slice',
		'pie',
		'brown',
		['chicken', 'pea', 'carrot'],
		true
	),
	d(
		'chocolate-cake',
		'Chocolate cake',
		'chocolate cake|schokoladenkuchen|schokokuchen|lava cake|sachertorte|schokotorte|schokoladentorte|schoko kuchen|schokoladen kuchen|schoko torte|schokoladen torte|chocolate torte|chocolate sponge|chocolate fudge cake|schoko fudge kuchen|fudge cake|fudge kuchen|fudge|chocolate lava cake|schokoladen lava kuchen|schoko lava kuchen|lava kuchen|schokoladenfondant|fondant au chocolat|fondant|chocolate fondant|molten chocolate cake|molten cake|flussiger schokokuchen|flussiger kern|flussiger schokoladenkuchen|schokoladenkuchen mit flussigem kern|schokokuchen mit flussigem kern|chocolate pudding cake|schokopudding kuchen|schokoladenpudding kuchen|brownie cake|brownie kuchen|brownie torte|brownie tart|brownie tarte|mud cake|mudcake|mud pie|devil s food cake|devils food cake|black forest cake|schwarzwalder kirschtorte|schwarzwaelder kirschtorte|schwarzwälder kirschtorte|schwarzwalder|black forest|kirsch sahne torte|cherry cream cake|cherry chocolate cake|kirsch schoko kuchen|kirsch schoko torte|schoko kirsch kuchen|schoko kirsch torte|chocolate cherry cake|chocolate cherry torte|chocolate orange cake|schoko orangen kuchen|schoko orangen torte|orangen schoko kuchen|orangen schoko torte|orange chocolate cake|orange chocolate torte|chocolate banana cake|schoko bananen kuchen|schoko bananen torte|bananen schoko kuchen|bananen schoko torte|banana chocolate cake|banana chocolate torte|chocolate zucchini cake|schoko zucchini kuchen|zucchini schoko kuchen|zucchini chocolate cake|chocolate beet cake|schoko rote bete kuchen|rote bete schoko kuchen|beetroot chocolate cake|chocolate avocado cake|schoko avocado kuchen|avocado schoko kuchen|avocado chocolate cake|chocolate courgette cake|schoko courgette kuchen|courgette schoko kuchen|courgette chocolate cake',
		'slice',
		'cake',
		'dark',
		['berry', 'swoosh'],
		true
	),
	d(
		'cheesecake',
		'Cheesecake',
		'cheesecake|kasekuchen|new york cheesecake|quarkkuchen|kase kuchen|kasetorte|kase torte|cheese cake|cheese torte|quark cake|quark torte|quarktorte|topfenkuchen|topfentorte|baked cheesecake|gebackener kasekuchen|no bake cheesecake|kasekuchen ohne backen|philadelphia torte|philadelphia kuchen|frischkase torte|frischkasetorte|frischkase kuchen|frischkasekuchen|cream cheese cake|cream cheese torte|cream cheese tart|frischkase tarte|frischkasetarte|cheesecake mit|cheesecake with|cheesecake cups|cheesecake im glas|cheesecake bars|cheesecake riegel|cheesecake brownies|cheesecake bites|cheesecake bällchen|cheesecake baellchen|cheesecake ballchen|cheesecake balls|cheesecake pops|cheesecake cake pops|cheesecake truffles|cheesecake trüffel|cheesecake truffel|cheesecake truffle|cheesecake trufflel',
		'slice',
		'cake',
		'milk',
		['berry', 'swoosh'],
		false
	),
	d(
		'cake',
		'Cake',
		'cake|kuchen|torte|sponge|victoria sponge|marmorkuchen|ruhrkuchen|rührkuchen|sandkuchen|sand kuchen|gugelhupf|gugelhopf|kastenkuchen|kasten kuchen|blechkuchen|blech kuchen|sheet cake|layer cake|schichttorte|schicht torte|sahnetorte|sahne torte|cream cake|cream torte|buttercreme torte|buttercremetorte|buttercream cake|buttercream torte|carrot cake|karottenkuchen|karotten kuchen|rubli kuchen|rubelikuchen|ruebli kuchen|rueblikuchen|carrot torte|karottentorte|karotten torte|lemon drizzle|lemon drizzle cake|orange cake|orangenkuchen|orangen kuchen|orange torte|orangentorte|orangen torte|nusskuchen|nuss kuchen|nut cake|nusstorte|nuss torte|nut torte|mohnkuchen|mohn kuchen|poppy seed cake|mohntorte|mohn torte|poppy seed torte|honigkuchen|honig kuchen|honey cake|lebkuchen|lebkuchen kuchen|gingerbread|gingerbread cake|ginger cake|ingwerkuchen|ingwer kuchen|spice cake|gewurzkuchen|gewurz kuchen|zimtkuchen|zimt kuchen|cinnamon cake|zimtschnecken|zimtschnecke|cinnamon rolls|cinnamon roll|cinnamon buns|cinnamon bun|kanelbullar|kanelbulle|kardamom schnecken|kardamomschnecken|cardamom buns|cardamom bun|hefeteig|yeast dough|hefekuchen|hefe kuchen|yeast cake|hefezopf kuchen|hefekranz|hefe kranz|yeast wreath|yeast ring|hefering|hefe ring|dessert|nachtisch|nachspeise|sweet|susses|suss|sussspeise|suss speise|sweet treat|sweet treats|treat|treats|sweets|susswaren|suss waren|leckerei|leckereien',
		'slice',
		'cake',
		'cream',
		['berry', 'sprinkle'],
		true
	),
	d(
		'brownies',
		'Brownies',
		'brownie|brownies|blondie|blondies|brownie bites|brownie bar|brownie bars|brownie squares|brownie schnitten|brownie schnitte|schoko brownies|schokoladen brownies|chocolate brownies|walnut brownies|walnuss brownies|nuss brownies|nut brownies|vegan brownies|black bean brownies|schwarze bohnen brownies|bohnen brownies|bean brownies|beetroot brownies|rote bete brownies|zucchini brownies|susskartoffel brownies|sweet potato brownies|avocado brownies|kichererbsen brownies|chickpea brownies|dattel brownies|date brownies|erdnuss brownies|peanut brownies|peanut butter brownies|erdnussbutter brownies|cookie dough brownies|keks brownies|oreo brownies|karamell brownies|caramel brownies|salted caramel brownies|gesalzene karamell brownies|marshmallow brownies|s mores brownies|smores brownies|raspberry brownies|himbeer brownies|cherry brownies|kirsch brownies|orange brownies|orangen brownies|mint brownies|minz brownies|cheesecake brownies kuchen|swirl brownies|marmor brownies|marble brownies|fudgy brownies|fudge brownies|chewy brownies|cakey brownies|double chocolate brownies|doppelt schoko brownies|triple chocolate brownies|dreifach schoko brownies|white chocolate brownies|weisse schoko brownies|weiße schoko brownies|dark chocolate brownies|zartbitter brownies|bitterschokolade brownies|milk chocolate brownies|vollmilch brownies|vollmilchschokolade brownies',
		'slice',
		'cake',
		'dark',
		['nut', 'dot'],
		false
	),
	d(
		'muffins',
		'Muffins',
		'muffin|muffins|cupcake|cupcakes|blueberry muffins|heidelbeermuffins|schoko muffins|chocolate muffins|bananen muffins|banana muffins|apfel muffins|apple muffins|apple cinnamon muffins|apfel zimt muffins|karotten muffins|carrot muffins|zucchini muffins|kurbis muffins|pumpkin muffins|himbeer muffins|raspberry muffins|erdbeer muffins|strawberry muffins|kirsch muffins|cherry muffins|zitronen muffins|lemon muffins|orangen muffins|orange muffins|mohn muffins|poppy seed muffins|nuss muffins|nut muffins|kase muffins|cheese muffins|herzhafte muffins|savoury muffins|savory muffins|schinken kase muffins|ham and cheese muffins|spinat feta muffins|spinach feta muffins|pizza muffins|gemuse muffins|vegetable muffins|veggie muffins|egg muffins pan|rührei muffins|ruhrei muffins|frittata muffins|mini muffins|mini cupcakes|mini kuchen|mini cakes|little cakes|kleine kuchen|kleiner kuchen|kleines gebaeck|kleines gebäck|kleines geback|small cakes|small cake|tortelettes|tortelette|tartelettes|tartelette|cake pops|cakepops|cake pop|cakepop|madeleines|madeleine|financiers|financier|friands|friand|fairy cakes|fairy cake|butterfly cakes|butterfly cake|whoopie pies|whoopie pie|whoopies|whoopie|macarons|macaron|macarons mit|macaroons|macaroon|makronen|makrone|kokosmakronen|kokosmakrone|coconut macaroons|coconut macaroon|mandelmakronen|mandelmakrone|almond macaroons|almond macaroon|nussmakronen|nussmakrone|nut macaroons|nut macaroon|haselnussmakronen|haselnussmakrone|hazelnut macaroons|hazelnut macaroon|schokomakronen|schokomakrone|chocolate macaroons|chocolate macaroon',
		'cup',
		'cake',
		'brown',
		['berry'],
		true
	),
	d(
		'banana-bread',
		'Banana bread',
		'banana bread|bananenbrot|bananenkuchen|banana cake|bananen brot|bananen kuchen|bananentorte|bananen torte|banana torte|banana loaf|bananen laib|bananenlaib|banana nut bread|bananen nuss brot|bananen nussbrot|bananen nuss kuchen|bananen nusskuchen|banana nut cake|banana walnut bread|bananen walnuss brot|bananen walnussbrot|bananen walnuss kuchen|bananen walnusskuchen|banana walnut cake|banana chocolate bread|bananen schoko brot|bananen schokobrot|bananen schoko kuchen|bananen schokokuchen|banana chocolate cake loaf|banana chocolate chip bread|bananen schokotropfen brot|bananen schokotropfenbrot|bananen schokotropfen kuchen|bananen schokotropfenkuchen|banana chocolate chip cake|banana oat bread|bananen hafer brot|bananen haferbrot|bananen hafer kuchen|bananen haferkuchen|banana oat cake|banana oatmeal bread|bananen haferflocken brot|bananen haferflockenbrot|bananen haferflocken kuchen|bananen haferflockenkuchen|banana oatmeal cake|zucchini bread|zucchinibrot|zucchini brot|zucchini kuchen|zucchinikuchen|zucchini cake|zucchini loaf|zucchini laib|zucchinilaib|pumpkin bread|kurbisbrot|kurbis brot|pumpkin loaf|kurbis laib|kurbislaib|carrot bread|karottenbrot|karotten brot|carrot loaf|karotten laib|karottenlaib|apple bread|apfelbrot|apfel brot|apple loaf|apfel laib|apfellaib|lemon loaf|zitronenbrot|zitronen brot|zitronen laib|zitronenlaib|lemon bread|orange loaf|orangenbrot|orangen brot|orangen laib|orangenlaib|orange bread|date loaf|dattelbrot|dattel brot|dattel laib|dattellaib|date bread|date nut loaf|dattel nuss brot|dattel nussbrot|dattel nuss laib|dattel nusslaib|date nut bread|fruit loaf|fruchtebrot|fruchte brot|fruchtebrot|fruechtebrot|früchtebrot|fruit bread|fruit cake|fruchtekuchen|fruchte kuchen|fruechtekuchen|früchtekuchen|christmas cake|weihnachtskuchen|weihnachts kuchen|christmas loaf|weihnachtsbrot|weihnachts brot|stollen|christstollen|dresdner stollen|dresden stollen|butterstollen|butter stollen|marzipanstollen|marzipan stollen|mohnstollen|mohn stollen|nussstollen|nuss stollen|quarkstollen|quark stollen|panettone|pandoro|colomba|colomba di pasqua|osterbrot|oster brot|easter bread|easter loaf|osterlaib|oster laib|osterzopf|oster zopf|easter braid|easter plait',
		'loaf',
		'cake',
		'brown',
		['banana', 'nut'],
		false
	),
	d(
		'tiramisu',
		'Tiramisu',
		'tiramisu|tiramisù|tiramisu im glas|tiramisu glass|erdbeer tiramisu|strawberry tiramisu|himbeer tiramisu|raspberry tiramisu|kaffee tiramisu|coffee tiramisu|schoko tiramisu|chocolate tiramisu|zitronen tiramisu|lemon tiramisu|orangen tiramisu|orange tiramisu|mango tiramisu|mascarpone|mascarpone creme|mascarpone cream|mascarpone dessert|mascarpone nachtisch|mascarpone nachspeise|mascarpone torte|mascarpone kuchen|mascarpone cake|mascarpone tart|mascarpone tarte|zabaione|zabaglione|panna cotta tiramisu|semifreddo|semi freddo|parfait glace|eisparfait|eis parfait|ice cream parfait|eistorte|eis torte|ice cream cake|ice cream torte|eiskuchen|eis kuchen|eis dessert|ice cream dessert|eisdessert|eis nachtisch|eisnachtisch|ice cream|eis|speiseeis|speise eis|gelato|sorbet|sorbetto|granita|frozen yoghurt|frozen yogurt|froyo|joghurteis|joghurt eis|yoghurt ice cream|yogurt ice cream|nice cream|nicecream|banana ice cream|bananeneis|bananen eis|bananen nicecream|bananen nice cream|bananenmousse|bananen mousse|banana mousse|schokomousse|schoko mousse|chocolate mousse|mousse au chocolat|mousse|mousses|mousse dessert|mousse nachtisch|mousse nachspeise|creme|cremes|creme dessert|creme nachtisch|creme nachspeise|creme brulee|creme caramel|karamellcreme|karamell creme|caramel cream|caramel custard|custard|vanillesosse|vanille sosse|vanilla sauce|vanilla custard|vanillecreme|vanille creme|vanilla cream|vanillepudding|vanille pudding|vanilla pudding|schokopudding|schoko pudding|chocolate pudding|pudding|puddings|puddingcreme|pudding creme|pudding cream|grießpudding|griesspudding|grieß pudding|griess pudding|semolina pudding|griesbrei pudding|grieskoch pudding|milchpudding|milch pudding|milk pudding|sago pudding|sagopudding|tapioca pudding|tapiokapudding|tapioka pudding|reispudding pudding|rice pudding pudding|chia pudding pudding|bread pudding|brotpudding|brot pudding|semmelpudding|semmel pudding|bread and butter pudding|yorkshire pudding|yorkshire|sticky toffee pudding|toffee pudding|christmas pudding|plum pudding pudding|steamed pudding|dampfpudding|dampf pudding|sponge pudding|sirup pudding|syrup pudding|treacle pudding|treacle sponge|jam roly poly|roly poly|spotted dick|summer pudding|sommerpudding|sommer pudding|sommerdessert|sommer dessert|sommernachtisch|sommer nachtisch|sommernachspeise|sommer nachspeise|rote grutze|rote grütze|rotgrutze|rotgrütze|rot grutze|rot grütze|red berry pudding|red fruit jelly|red fruit compote|beerengrutze|beerengrütze|beeren grutze|beeren grütze|berry compote|beerenkompott|beeren kompott|kompott|compote|fruit compote|fruchtkompott|frucht kompott|obstkompott|obst kompott|apfelkompott|apfel kompott|apple compote|apple sauce|apfelmus|apfel mus|apple puree|apfelpuree|apfel puree|pflaumenkompott|pflaumen kompott|plum compote|zwetschgenrost|zwetschgen rost|zwetschgenkompott|zwetschgen kompott|rhabarberkompott|rhabarber kompott|rhubarb compote|erdbeerkompott|erdbeer kompott|strawberry compote|kirschkompott|kirsch kompott|cherry compote|birnenkompott|birnen kompott|pear compote|pfirsichkompott|pfirsich kompott|peach compote|mangokompott|mango kompott|mango compote|ananaskompott|ananas kompott|pineapple compote',
		'slice',
		'cake',
		'cream',
		['sprinkle'],
		false
	),
	d(
		'rice-pudding',
		'Rice pudding',
		'rice pudding|milchreis|reispudding|reis pudding|milch reis|reisbrei suss|susser reisbrei|sweet rice|susser reis|suss reis|sweet rice pudding|susser milchreis|milchreis mit|rice pudding with|reispudding mit|milchreis mit zimt|milchreis mit zucker|milchreis mit apfelmus|milchreis mit kirschen|milchreis mit fruchten|milchreis mit obst|milchreis mit kompott|rice pudding with compote|rice pudding with fruit|rice pudding with cinnamon|rice pudding with jam|milchreis mit marmelade|milchreis mit konfture|milchreis mit konfitüre|milchreis mit konfiture|arroz con leche|riz au lait|kheer|payasam|sholeh zard|ris a la malta|risalamande|risengrynsgrod|risengrynsgrød|riisipuuro|riskrem|grot|grøt|rice grot|rice grøt|rice cream|reiscreme|reis creme|reisdessert|reis dessert|reisnachtisch|reis nachtisch|reisnachspeise|reis nachspeise|reisauflauf suss|susser reisauflauf|sweet rice bake|sweet rice casserole|reis auflauf suss|susser reis auflauf|rice bake sweet|rice casserole sweet',
		'small',
		'porridge',
		'milk',
		['sprinkle', 'apple'],
		true
	),
	d(
		'fruit-salad',
		'Fruit salad',
		'fruit salad|obstsalat|fruchtsalat|obst salat|frucht salat|fruit bowl|obstschale|obst schale|obstschussel|obst schussel|fruchteschale|fruchte schale|fruchteschussel|fruchte schussel|fresh fruit|frisches obst|fruit plate|obstteller|obst teller|fruchteteller|fruchte teller|fruit platter|obstplatte|obst platte|fruchteplatte|fruchte platte|fruit cup|fruit cups|obstbecher|obst becher|fruchtebecher|fruchte becher|macedonia|macedoine|macedoine de fruits|salade de fruits|ambrosia|ambrosia salad|waldorf fruit|tropical fruit salad|tropischer obstsalat|exotic fruit salad|exotischer obstsalat|exotischer fruchtsalat|summer fruit salad|sommer obstsalat|sommerlicher obstsalat|winter fruit salad|winter obstsalat|winterlicher obstsalat|autumn fruit salad|herbst obstsalat|herbstlicher obstsalat|spring fruit salad|fruhlings obstsalat|fruhlingshafter obstsalat|berry salad|beerensalat|beeren salat|melon fruit salad|melonen obstsalat|citrus salad|zitrus salat|zitrussalat|orange salad|orangensalat|orangen salat|orange and fennel salad|orangen fenchel salat|orangen fenchelsalat|grapefruit salad|grapefruitsalat|grapefruit salat|mango salad|mangosalat|mango salat|pineapple salad|ananassalat|ananas salat|banana salad|bananensalat|bananen salat|apple salad fruit|pear salad|birnensalat|birnen salat|peach salad|pfirsichsalat|pfirsich salat|plum salad|pflaumensalat|pflaumen salat|cherry salad|kirschsalat|kirsch salat|grape salad|traubensalat|trauben salat|kiwi salad|kiwisalat|kiwi salat|strawberry salad fruit',
		'bowl',
		'none',
		'cream',
		['apple', 'banana', 'berry', 'citrus'],
		false
	),
	d(
		'chia-pudding',
		'Chia pudding',
		'chia pudding|chia|chiapudding|chia pudding mit|chia seed pudding|chia samen pudding|chia samenpudding|chia dessert|chia nachtisch|chia nachspeise|chia fruhstuck|chia breakfast|chia bowl|chia schussel|chia glas|chia jar|chia im glas|chia mit|chia with|chia kokos|chia coconut|coconut chia pudding|kokos chia pudding|kokoschia pudding|kokos chiapudding|mango chia pudding|mango chiapudding|chia mango|chia beeren|chia berries|berry chia pudding|beeren chia pudding|beeren chiapudding|chocolate chia pudding|schoko chia pudding|schoko chiapudding|schokolade chia pudding|schokolade chiapudding|vanilla chia pudding|vanille chia pudding|vanille chiapudding|matcha chia pudding|matcha chiapudding|banana chia pudding|bananen chia pudding|bananen chiapudding|peanut chia pudding|erdnuss chia pudding|erdnuss chiapudding|peanut butter chia pudding|erdnussbutter chia pudding|erdnussbutter chiapudding',
		'jar',
		'yoghurt',
		'milk',
		['berry', 'seed'],
		false
	),
	d(
		'panna-cotta',
		'Panna cotta',
		'panna cotta|pannacotta|panacotta|panna cotta mit|panna cotta with|vanille panna cotta|vanilla panna cotta|erdbeer panna cotta|strawberry panna cotta|himbeer panna cotta|raspberry panna cotta|beeren panna cotta|berry panna cotta|mango panna cotta|kokos panna cotta|coconut panna cotta|schoko panna cotta|chocolate panna cotta|karamell panna cotta|caramel panna cotta|kaffee panna cotta|coffee panna cotta|joghurt panna cotta|yoghurt panna cotta|yogurt panna cotta|buttermilch panna cotta|buttermilk panna cotta|zitronen panna cotta|lemon panna cotta|orangen panna cotta|orange panna cotta|vegan panna cotta|veganes panna cotta|vegane panna cotta|gelatine dessert|gelatine nachtisch|gelatine nachspeise|wackelpudding|jelly|jello|gelee|gelees|jellies|gelatin|gelatin dessert|gelatin nachtisch|gelatin nachspeise|blancmange|blanc manger|flan|flans|creme flan|flan caramel|karamellflan|karamell flan|caramel flan|crème caramel|creme renversee|crème renversée|bavarois|bayerische creme|bavarian cream|bayrische creme|bayerisch creme|bayrisch creme|charlotte|charlotte russe|charlotte royale|charlotte aux fraises|charlotte aux poires|charlotte aux pommes|charlotte aux framboises|charlotte aux fruits rouges|charlotte aux fruits|charlotte aux chocolat|charlotte au chocolat|charlotte au cafe|charlotte au café|charlotte au caramel|charlotte a la vanille|charlotte à la vanille|charlotte a la fraise|charlotte à la fraise|charlotte a la framboise|charlotte à la framboise|charlotte a la poire|charlotte à la poire|charlotte a la pomme|charlotte à la pomme',
		'glass',
		'yoghurt',
		'cream',
		['berry', 'swoosh'],
		false
	),
	d(
		'cookies',
		'Cookies',
		'cookie|cookies|kekse|keks|platzchen|plätzchen|plaetzchen|biscuit|biscuits|haferkekse|haferflockenkekse|oatmeal cookies|oat cookies|oatmeal raisin cookies|schoko cookies|chocolate chip cookies|choc chip cookies|schokotropfen kekse|schokoladen kekse|schoko kekse|chocolate cookies|double chocolate cookies|erdnuss cookies|peanut butter cookies|erdnussbutter cookies|erdnussbutter kekse|erdnuss kekse|peanut cookies|mandel kekse|almond cookies|mandelplatzchen|mandel platzchen|nuss kekse|nut cookies|nussplatzchen|nuss platzchen|vanillekipferl|vanille kipferl|kipferl|kipferln|spritzgebaeck|spritzgebäck|spritzgeback|spritz gebaeck|spritz gebäck|spritz geback|butterplatzchen|butter platzchen|butter cookies|butterkekse|butter kekse|shortbread|short bread|shortbread cookies|shortbread kekse|zimtsterne|zimt sterne|cinnamon stars|zimtplatzchen|zimt platzchen|cinnamon cookies|zimtkekse|zimt kekse|lebkuchen kekse|lebkuchen platzchen|gingerbread cookies|gingerbread men|gingerbread man|lebkuchenmanner|lebkuchenmännchen|lebkuchenmaennchen|lebkuchenmann|ausstecherle|ausstecher|ausstechplatzchen|ausstech platzchen|cookie cutter cookies|cut out cookies|sugar cookies|zuckerkekse|zucker kekse|zuckerplatzchen|zucker platzchen|linzer|linzer augen|linzer augen kekse|linzer kekse|linzer platzchen|linzer cookies|linzer torte|linzertorte|spitzbuben|spitzbub|jam cookies|marmeladenkekse|marmeladen kekse|marmeladenplatzchen|marmeladen platzchen|thumbprint cookies|thumbprint|kokosmakronen cookies|coconut cookies|kokoskekse|kokos kekse|kokosplatzchen|kokos platzchen|florentiner|florentines|florentine|haferflockenplatzchen|haferflocken platzchen|hafer kekse|hafer platzchen|hafer cookies|energy cookies|energie kekse|energiekekse|fruhstuckskekse|fruhstucks kekse|breakfast cookies|banana oat cookies|bananen hafer kekse|bananen haferkekse|bananen hafer cookies|2 ingredient cookies|zwei zutaten kekse|2 zutaten kekse|zwei zutaten cookies|2 zutaten cookies|biscotti|cantucci|cantuccini|amaretti|amaretti kekse|amaretti cookies|amarettini|amaretti platzchen|ladyfingers|savoiardi|loffelbiskuit|löffelbiskuit|loeffelbiskuit|biskuit|biskuit kekse|biskuitplatzchen|biskuit platzchen|sponge fingers|sponge biscuits|crackers|cracker|knackebrot|knäckebrot|knaeckebrot|knackebrot selber machen|crispbread|crisp bread|seed crackers|samen cracker|samencracker|samenkracker|samen kracker|nuss cracker|nusscracker|nusskracker|nuss kracker|nut crackers|nut cracker|reiscracker|reis cracker|rice crackers|rice cracker|reiskracker|reis kracker|salzstangen|salz stangen|salt sticks|salted sticks|brezel|brezeln|pretzel|pretzels|laugenbrezel|laugenbrezeln|laugen brezel|laugen brezeln|laugenstange|laugenstangen|laugen stange|laugen stangen|laugenbrotchen|laugenbrötchen|laugenbroetchen|laugen brotchen|laugen brötchen|laugen broetchen|laugenzopf|laugen zopf|laugenhorn|laugen horn|laugenhornchen|laugenhörnchen|laugenhoernchen|laugen hornchen|laugen hörnchen|laugen hoernchen|croissant|croissants|hornchen|hörnchen|hoernchen|butterhornchen|butterhörnchen|butterhoernchen|butter hornchen|butter hörnchen|butter hoernchen|schokocroissant|schoko croissant|chocolate croissant|pain au chocolat|schokobrotchen|schoko brötchen|schoko broetchen|schoko brotchen|chocolate roll|chocolate rolls|rosinenbrotchen|rosinen brötchen|rosinen broetchen|rosinen brotchen|raisin rolls|raisin roll|raisin buns|raisin bun|rosinenschnecke|rosinen schnecke|rosinenschnecken|rosinen schnecken|raisin snail|raisin snails|raisin swirl|raisin swirls|raisin pinwheel|raisin pinwheels|raisin rolls swirl|raisin rolls swirls',
		'plate',
		'patty',
		'brown',
		['dot', 'nut'],
		false
	)
];

/** The generic fallback when nothing matches. */
export const FALLBACK: DishKind = {
	id: 'bowl',
	label: 'A bowl of something good',
	words: [],
	vessel: 'bowl',
	base: 'grain',
	tint: 'wheat',
	signature: [],
	hot: true
};

export function byId(id: string): DishKind | undefined {
	return DISHES.find((d) => d.id === id);
}

// ---- matching ----

/**
 * Fold a text to comparable tokens: lower case, no accents, `ae/oe/ue` = `a/o/u`,
 * `ss` for ß, punctuation = space, filler words and 1-letter words dropped,
 * a plural `s` removed (`eggs` = `egg`, `tomatoes` = `tomato`).
 */
export function norm(text: string): string[] {
	const s = text
		.toLowerCase()
		.replace(/ß/g, 'ss')
		.normalize('NFD')
		.replace(/[̀-ͯ]/g, '')
		.replace(/ae/g, 'a')
		.replace(/oe/g, 'o')
		.replace(/ue/g, 'u');
	const out: string[] = [];
	for (const raw of s.split(/[^a-z0-9]+/)) {
		if (raw.length < 2 || STOP.has(raw)) continue;
		out.push(stem(raw));
	}
	return out;
}

const STOP = new Set([
	'and',
	'with',
	'the',
	'of',
	'on',
	'in',
	'at',
	'for',
	'to',
	'an',
	'or',
	'mit',
	'und',
	'auf',
	'im',
	'in',
	'der',
	'die',
	'das',
	'dem',
	'den',
	'ein',
	'eine',
	'vom',
	'von',
	'aus',
	'an',
	'am',
	'de',
	'di',
	'al',
	'alla',
	'con'
]);

function stem(t: string): string {
	if (t.length > 3 && t.endsWith('s') && !/(ss|us|is)$/.test(t)) return t.slice(0, -1);
	return t;
}

/** Single words that name only a family: they match weakly (a specific phrase always wins). */
const GENERIC = new Set(
	[
		'bowl',
		'soup',
		'suppe',
		'salad',
		'salat',
		'curry',
		'stew',
		'bake',
		'baked',
		'casserole',
		'auflauf',
		'pasta',
		'nudeln',
		'nudel',
		'spaghetti',
		'penne',
		'fusilli',
		'tagliatelle',
		'rigatoni',
		'linguine',
		'farfalle',
		'macaroni',
		'makkaroni',
		'rice',
		'reis',
		'noodle',
		'sandwich',
		'pie',
		'cake',
		'kuchen',
		'torte',
		'toast',
		'wrap',
		'chicken',
		'hahnchen',
		'huhn',
		'fish',
		'fisch',
		'sausage',
		'wurst',
		'eintopf',
		'brot',
		'bread',
		'roast',
		'braten',
		'dessert',
		'pan',
		'pfanne',
		'schussel',
		'tortilla',
		'potato',
		'kartoffel',
		'bean',
		'roasted',
		'sweet',
		'dip',
		'stick',
		'tart',
		'tarte',
		'pudding',
		'creme',
		'eis',
		'pizza',
		'sauce',
		'sosse',
		'topf',
		'wok',
		'rolle',
		'filo',
		'pute',
		'turkey',
		'lamb',
		'lamm',
		'wild',
		'duck',
		'ente',
		'beetroot',
		'tomato',
		'tuna',
		'thunfisch',
		'cheese',
		'kase',
		'mash',
		'broth',
		'bruhe',
		'yoghurt',
		'joghurt',
		'yogurt',
		'quark',
		'oat',
		'hafer',
		'egg',
		'ei',
		'steak',
		'burger',
		'gemuse',
		'veg'
	].flatMap((w) => norm(w))
);

interface Entry {
	tokens: string[];
	/** Single weak word. */
	weak: boolean;
}

let index: { kind: DishKind; entries: Entry[] }[] | undefined;

function build() {
	if (index) return index;
	index = DISHES.map((kind) => {
		const seen = new Set<string>();
		const entries: Entry[] = [];
		for (const w of kind.words) {
			const tokens = norm(w);
			const key = tokens.join(' ');
			if (!tokens.length || seen.has(key)) continue;
			seen.add(key);
			entries.push({ tokens, weak: tokens.length === 1 && GENERIC.has(tokens[0]) });
		}
		return { kind, entries };
	});
	return index;
}

/** Index of `needle` as a run inside `hay`, or -1. */
function runAt(hay: string[], needle: string[]): number {
	outer: for (let i = 0; i + needle.length <= hay.length; i++) {
		for (let j = 0; j < needle.length; j++) if (hay[i + j] !== needle[j]) continue outer;
		return i;
	}
	return -1;
}

const WEAK = 5;
const PER_TOKEN = 20;

/**
 * Score of one phrase: a run of its tokens in the title counts fully
 * (20 per token, 5 for a lone family word like "soup"); all tokens somewhere
 * in the title 0.6 of that; some in the title and the rest only among the
 * ingredients 0.3 ("soup" + lentils -> lentil soup). Earlier mentions win ties.
 */
function phraseScore(e: Entry, title: string[], ings: Set<string>): number {
	const n = e.tokens.length;
	const full = e.weak ? WEAK : n * PER_TOKEN;
	const at = runAt(title, e.tokens);
	if (at >= 0) return full - at * 0.01;
	if (n < 2) return 0;
	let inTitle = 0;
	let inIngs = 0;
	let first = Infinity;
	for (const t of e.tokens) {
		const i = title.indexOf(t);
		if (i >= 0) {
			inTitle++;
			first = Math.min(first, i);
		} else if (ings.has(t)) inIngs++;
	}
	if (inTitle === n) return full * 0.6 - first * 0.01;
	if (inTitle >= 1 && inTitle + inIngs === n) return full * 0.3 - first * 0.01;
	return 0;
}

/** Ingredient-only hints for titles with no dish word: all groups must hit. */
const RULES: [string[][], string][] = [
	[[['egg'], ['tomato']], 'shakshuka'],
	[
		[
			['spaghetti', 'penne', 'pasta', 'nudel', 'nudeln', 'fusilli', 'tagliatelle', 'rigatoni'],
			['tomato']
		],
		'pasta-pomodoro'
	],
	[
		[['spaghetti', 'penne', 'pasta', 'nudel', 'nudeln', 'fusilli', 'tagliatelle', 'rigatoni']],
		'pasta'
	],
	[
		[
			['lentil', 'linse'],
			['carrot', 'karotte']
		],
		'lentil-soup'
	],
	[
		[
			['chickpea', 'kichererbse'],
			['rice', 'reis']
		],
		'chickpea-curry'
	],
	[[['rice', 'reis']], 'rice-bowl'],
	[[['noodle', 'ramen', 'udon']], 'chow-mein'],
	[[['lentil', 'linse']], 'lentil-soup'],
	[[['chickpea', 'kichererbse']], 'chickpea-curry'],
	[
		[
			['potato', 'kartoffel'],
			['sausage', 'wurst']
		],
		'sausage-mash'
	],
	[
		[
			['potato', 'kartoffel'],
			['egg', 'ei']
		],
		'bratkartoffeln'
	],
	[[['tortilla', 'wrap']], 'wrap'],
	[[['bread', 'toast', 'brot', 'baguette']], 'toast'],
	[[['mince', 'hackfleisch']], 'meatballs'],
	[[['salmon', 'lachs']], 'salmon'],
	[[['chicken', 'hahnchen']], 'roast-chicken'],
	[[['egg', 'ei']], 'scrambled-eggs'],
	[[['oat', 'hafer']], 'porridge'],
	[[['potato', 'kartoffel']], 'baked-potato']
];

function byRule(ings: Set<string>): DishKind | undefined {
	for (const [groups, id] of RULES) {
		const hit = groups.every((g) => g.some((t) => ings.has(stem(t)) || ings.has(t)));
		if (hit) return byId(id);
	}
	return undefined;
}

/**
 * The best kind for a recipe, from its title (English or German) and its
 * ingredient names. Deterministic.
 *
 * Every kind scores its words against the title (see `phraseScore`: longer
 * phrases beat shorter ones, a lone family word like "soup" is weak); the
 * ingredients add a little to break ties. A title with no dish word falls back
 * to ingredient rules (eggs + tomato = shakshuka, pasta = pasta, rice = rice
 * bowl ...), then to `FALLBACK`.
 */
export function matchDish(title: string, ingredients: readonly string[] = []): DishKind {
	const t = norm(title);
	const ings = new Set(ingredients.flatMap((i) => norm(i)));
	let best: DishKind | undefined;
	let bestScore = 0;
	for (const { kind, entries } of build()) {
		let score = 0;
		for (const e of entries) score = Math.max(score, phraseScore(e, t, ings));
		if (score <= 0) continue;
		// ingredients break ties: each ingredient that is one of the kind's own words
		let bonus = 0;
		for (const e of entries) if (e.tokens.length === 1 && ings.has(e.tokens[0])) bonus += 0.5;
		score += Math.min(bonus, 2);
		if (score > bestScore) {
			bestScore = score;
			best = kind;
		}
	}
	if (best) return best;
	return byRule(ings) ?? FALLBACK;
}
