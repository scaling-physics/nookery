//! Quantities in integer thousandths, so halves and 1.5 kg add up exactly.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    /// Countable things: 6 eggs, half a cucumber.
    Piece,
    Gram,
    Millilitre,
}

/// An amount in thousandths of its unit (1 piece = 1000, 1 g = 1000).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Qty {
    pub milli: i64,
    pub unit: Unit,
}

impl Qty {
    pub const fn pieces(n: i64) -> Self {
        Qty {
            milli: n * 1000,
            unit: Unit::Piece,
        }
    }
    pub const fn grams(n: i64) -> Self {
        Qty {
            milli: n * 1000,
            unit: Unit::Gram,
        }
    }
    pub const fn millilitres(n: i64) -> Self {
        Qty {
            milli: n * 1000,
            unit: Unit::Millilitre,
        }
    }
    pub fn is_zero(&self) -> bool {
        self.milli <= 0
    }
    /// `self - other`, never below zero; None if the units differ.
    pub fn minus(self, other: Qty) -> Option<Qty> {
        (self.unit == other.unit).then(|| Qty {
            milli: (self.milli - other.milli).max(0),
            unit: self.unit,
        })
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("can't read the amount {0:?} (try 2, 0.5, 500g, 1.5kg, 750ml, 1l)")]
pub struct QtyParseError(pub String);

impl FromStr for Qty {
    type Err = QtyParseError;

    /// `2`, `0.5`, `2pc`, `500g`, `1.5kg`, `750ml`, `1l`, `1,5 kg` (comma too).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || QtyParseError(s.to_string());
        let t = s.trim().to_ascii_lowercase().replace(',', ".");
        let split = t
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(t.len());
        let (num, unit) = t.split_at(split);
        let (unit, factor) = match unit.trim() {
            "" | "pc" | "pcs" | "x" | "st" | "stk" => (Unit::Piece, 1000),
            "g" => (Unit::Gram, 1000),
            "kg" => (Unit::Gram, 1_000_000),
            "ml" => (Unit::Millilitre, 1000),
            "cl" => (Unit::Millilitre, 10_000),
            "l" => (Unit::Millilitre, 1_000_000),
            _ => return Err(err()),
        };
        let milli = parse_scaled(num, factor).ok_or_else(err)?;
        if milli <= 0 {
            return Err(err());
        }
        Ok(Qty { milli, unit })
    }
}

/// `num * factor` for a decimal string, without floating point.
fn parse_scaled(num: &str, factor: i64) -> Option<i64> {
    let (whole, frac) = num.split_once('.').unwrap_or((num, ""));
    if whole.is_empty() && frac.is_empty() {
        return None;
    }
    let whole: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let mut value = whole.checked_mul(factor)?;
    let mut scale = factor;
    for c in frac.chars() {
        scale /= 10;
        if scale == 0 {
            break; // finer than a thousandth: drop
        }
        value += i64::from(c.to_digit(10)? as u8) * scale;
    }
    Some(value)
}

impl fmt::Display for Qty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (value, unit) = match self.unit {
            Unit::Piece => (self.milli, ""),
            Unit::Gram if self.milli >= 1_000_000 => (self.milli / 1000, "kg"),
            Unit::Gram => (self.milli, "g"),
            Unit::Millilitre if self.milli >= 1_000_000 => (self.milli / 1000, "l"),
            Unit::Millilitre => (self.milli, "ml"),
        };
        let whole = value / 1000;
        let frac = value % 1000;
        if frac == 0 {
            write!(f, "{whole}{unit}")
        } else {
            let frac = format!("{frac:03}");
            write!(f, "{whole}.{}{unit}", frac.trim_end_matches('0'))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_amounts() {
        assert_eq!("2".parse(), Ok(Qty::pieces(2)));
        assert_eq!(
            "0.5".parse(),
            Ok(Qty {
                milli: 500,
                unit: Unit::Piece
            })
        );
        assert_eq!("500g".parse(), Ok(Qty::grams(500)));
        assert_eq!("1,5 kg".parse(), Ok(Qty::grams(1500)));
        assert_eq!("750ml".parse(), Ok(Qty::millilitres(750)));
        assert_eq!("1L".parse(), Ok(Qty::millilitres(1000)));
        assert_eq!("6 stk".parse(), Ok(Qty::pieces(6)));
    }

    #[test]
    fn rejects_nonsense() {
        assert!("".parse::<Qty>().is_err());
        assert!("0".parse::<Qty>().is_err());
        assert!("2 bananas".parse::<Qty>().is_err());
        assert!(".".parse::<Qty>().is_err());
    }

    #[test]
    fn displays_compactly() {
        assert_eq!(Qty::grams(1500).to_string(), "1.5kg");
        assert_eq!(Qty::grams(250).to_string(), "250g");
        assert_eq!(Qty::millilitres(1000).to_string(), "1l");
        assert_eq!(
            Qty {
                milli: 500,
                unit: Unit::Piece
            }
            .to_string(),
            "0.5"
        );
        assert_eq!(Qty::pieces(6).to_string(), "6");
    }

    #[test]
    fn minus_never_goes_negative_and_checks_units() {
        assert_eq!(Qty::pieces(2).minus(Qty::pieces(3)), Some(Qty::pieces(0)));
        assert_eq!(Qty::pieces(2).minus(Qty::grams(1)), None);
    }
}
