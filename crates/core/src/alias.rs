//! What a receipt line means, learned from the user's confirmations: per
//! store, the printed text ("BIO H-MILCH 1,5%") maps to a product ("milk",
//! 1 l each, fridge). The next receipt with the same text needs no question.

use serde::{Deserialize, Serialize};

use crate::{Location, Qty};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alias {
    /// The product as the user calls it, e.g. "milk".
    pub item: String,
    /// The amount of one bought unit (1 l of milk, 6 eggs, 500 g of quark).
    /// None for weighed goods, whose amount the receipt prints.
    pub each: Option<Qty>,
    pub location: Location,
}
