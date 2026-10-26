//! Mana cost parsing and pure payment planning.
//!
//! Planning is separated from mutation so that a failed payment can never
//! half-tap a board: validation builds a plan, mutation applies it.

use crate::card::ManaColor;
use crate::ids::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaCost {
    pub generic: u8,
    pub pips: Vec<ManaColor>,
}

impl ManaCost {
    pub fn mana_value(&self) -> u8 {
        self.generic + self.pips.len() as u8
    }

    pub fn render(&self) -> String {
        let mut s = String::new();
        if self.generic > 0 {
            s.push_str(&format!("{{{}}}", self.generic));
        }
        for pip in &self.pips {
            s.push_str(pip.symbol());
        }
        s
    }

    pub fn empty() -> Self {
        ManaCost {
            generic: 0,
            pips: Vec::new(),
        }
    }
}

/// Parse `"{2}{U}{U}"`-style costs. Unknown symbols are ignored rather than
/// guessed; the pool only uses generic and single-color pips.
pub fn parse_cost(text: &str) -> ManaCost {
    let mut cost = ManaCost::empty();
    for raw in text.split('{') {
        let symbol = raw.split('}').next().unwrap_or("").trim();
        match symbol {
            "" => {}
            "W" => cost.pips.push(ManaColor::White),
            "U" => cost.pips.push(ManaColor::Blue),
            "B" => cost.pips.push(ManaColor::Black),
            "R" => cost.pips.push(ManaColor::Red),
            "G" => cost.pips.push(ManaColor::Green),
            "C" => cost.pips.push(ManaColor::Colorless),
            _ => {
                if let Ok(n) = symbol.parse::<u8>() {
                    cost.generic = cost.generic.saturating_add(n);
                }
            }
        }
    }
    cost
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ManaPool {
    pub white: u32,
    pub blue: u32,
    pub black: u32,
    pub red: u32,
    pub green: u32,
    pub colorless: u32,
}

impl ManaPool {
    pub fn get(&self, color: ManaColor) -> u32 {
        match color {
            ManaColor::White => self.white,
            ManaColor::Blue => self.blue,
            ManaColor::Black => self.black,
            ManaColor::Red => self.red,
            ManaColor::Green => self.green,
            ManaColor::Colorless => self.colorless,
        }
    }

    pub fn add(&mut self, color: ManaColor, amount: u32) {
        let slot = match color {
            ManaColor::White => &mut self.white,
            ManaColor::Blue => &mut self.blue,
            ManaColor::Black => &mut self.black,
            ManaColor::Red => &mut self.red,
            ManaColor::Green => &mut self.green,
            ManaColor::Colorless => &mut self.colorless,
        };
        *slot = slot.saturating_add(amount);
    }

    pub fn take(&mut self, color: ManaColor, amount: u8) -> u8 {
        let slot = match color {
            ManaColor::White => &mut self.white,
            ManaColor::Blue => &mut self.blue,
            ManaColor::Black => &mut self.black,
            ManaColor::Red => &mut self.red,
            ManaColor::Green => &mut self.green,
            ManaColor::Colorless => &mut self.colorless,
        };
        let taken = (*slot).min(u32::from(amount));
        *slot -= taken;
        taken as u8
    }

    pub fn total(&self) -> u32 {
        self.white
            .saturating_add(self.blue)
            .saturating_add(self.black)
            .saturating_add(self.red)
            .saturating_add(self.green)
            .saturating_add(self.colorless)
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    /// Deterministic render, e.g. `{U}{U}`.
    pub fn render(&self) -> String {
        let mut s = String::new();
        for (color, n) in [
            (ManaColor::Colorless, self.colorless),
            (ManaColor::White, self.white),
            (ManaColor::Blue, self.blue),
            (ManaColor::Black, self.black),
            (ManaColor::Red, self.red),
            (ManaColor::Green, self.green),
        ] {
            for _ in 0..n {
                s.push_str(color.symbol());
            }
        }
        if s.is_empty() {
            "(empty)".to_string()
        } else {
            s
        }
    }
}

/// Mana available without tapping anything beyond what is listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sources {
    pub pool: ManaPool,
    /// Untapped lands that can be tapped for one mana each, in a deterministic
    /// (ascending object id) order.
    pub lands: Vec<(ObjectId, ManaColor)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payment {
    pub taps: Vec<ObjectId>,
    /// Mana left in the pool after payment (unspent mana is kept; it empties on
    /// the next step or phase change, CR 500.5).
    pub remaining: ManaPool,
}

/// Plan a legal payment, or `None` if the cost cannot be paid. Colors are
/// satisfied before generic mana; leftover pool mana is spent before new taps.
pub fn plan(sources: &Sources, cost: &ManaCost) -> Option<Payment> {
    let mut pool = sources.pool;
    let mut untapped = sources.lands.clone();
    let mut taps: Vec<ObjectId> = Vec::new();

    let mut requirements: Vec<(ManaColor, u8)> = Vec::new();
    for pip in &cost.pips {
        match requirements.iter_mut().find(|(c, _)| c == pip) {
            Some((_, n)) => *n += 1,
            None => requirements.push((*pip, 1)),
        }
    }

    for (color, needed) in requirements {
        let from_pool = pool.take(color, needed);
        let mut missing = needed - from_pool;
        while missing > 0 {
            let idx = untapped.iter().position(|(_, c)| *c == color)?;
            let (land, _) = untapped.remove(idx);
            taps.push(land);
            pool.add(color, 1);
            pool.take(color, 1);
            missing -= 1;
        }
    }

    let mut generic = cost.generic;
    for color in [
        ManaColor::Colorless,
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        while generic > 0 {
            if pool.take(color, 1) == 0 {
                break;
            }
            generic -= 1;
        }
    }
    while generic > 0 {
        if untapped.is_empty() {
            return None;
        }
        let (land, color) = untapped.remove(0);
        taps.push(land);
        pool.add(color, 1);
        pool.take(color, 1);
        generic -= 1;
    }

    Some(Payment {
        taps,
        remaining: pool,
    })
}
