//! Library operations (CR 401, 701.13, 701.16, 701.22, 701.25).
//!
//! These are the state-level rules primitives. Each one is deterministic given
//! the serialized state (so a replay reproduces it exactly) and none of them can
//! look at, or reveal, a card that the rules do not let the acting player see.
//!
//! Visibility rules encoded here:
//! * a library is hidden: only its size is public (CR 401.2);
//! * "reveal" makes the named card public while it stays in a hidden zone, and
//!   never moves it (CR 701.16a);
//! * milling moves cards from the top of the library to the graveyard, which is
//!   a public zone (CR 701.13a).
//!
//! Commander interaction is explicit rather than assumed: a commander card
//! milled from a library must be offered its CR 903.9a choice by the engine, so
//! [`GameState::mill`] stops before a commander instead of silently bypassing
//! that choice. The engine's draw path owns the replacement flow; mill is used
//! by library-mill effects, which no supported card has yet.

use crate::ids::{ObjectId, PlayerId, Zone};
use crate::state::{GameState, Object};

impl GameState {
    /// CR 701.13a: put the top `count` cards of `player`'s library into their
    /// graveyard, in library order. Returns the moved object ids, top first.
    ///
    /// Stops (and returns what moved) if the next card is a commander, because
    /// moving a commander into a graveyard may be replaced by CR 903.9a and that
    /// owner choice belongs to the engine, not to a state primitive.
    pub fn mill(&mut self, player: PlayerId, count: u32) -> Vec<ObjectId> {
        assert!(
            (player as usize) < self.players.len(),
            "mill seat P{player} is not a seat"
        );
        let mut moved = Vec::new();
        for _ in 0..count {
            let Some(&top) = self.players[player as usize].library.first() else {
                break;
            };
            let Some(object) = self.objects.get(&top) else {
                break;
            };
            if object.commander.is_some() || object.card.is_token() {
                break;
            }
            let mut object = self
                .objects
                .remove(&top)
                .expect("library object exists in state objects");
            self.players[player as usize].library.remove(0);
            object.zone = Zone::Graveyard;
            object.tapped = false;
            object.damage = 0;
            object.summoning_sick = false;
            object.pump_power = 0;
            object.pump_toughness = 0;
            object.counters.clear();
            self.players[player as usize].graveyard.push(top);
            self.objects.insert(top, object);
            moved.push(top);
        }
        moved
    }

    /// CR 701.20a: randomize the library with the game's deterministic RNG.
    /// The card multiset is unchanged; only the order moves.
    pub fn shuffle_library(&mut self, player: PlayerId) {
        assert!(
            (player as usize) < self.players.len(),
            "shuffle seat P{player} is not a seat"
        );
        let mut library = std::mem::take(&mut self.players[player as usize].library);
        self.rng.shuffle(&mut library);
        self.players[player as usize].library = library;
    }

    /// CR 701.16a: reveal the top `count` cards of `player`'s library without
    /// moving them. Returns the revealed ids, top first. Every player may see
    /// them while they stay in a hidden zone; the projection enforces that.
    pub fn reveal_top(&mut self, player: PlayerId, count: u32) -> Vec<ObjectId> {
        assert!(
            (player as usize) < self.players.len(),
            "reveal seat P{player} is not a seat"
        );
        // Drop reveals that already left the hidden zones so the list can never
        // keep claiming a card is public after it moved somewhere public.
        self.prune_revealed();
        let revealed: Vec<ObjectId> = self.players[player as usize]
            .library
            .iter()
            .take(count as usize)
            .copied()
            .collect();
        for id in &revealed {
            if !self.revealed.contains(id) {
                self.revealed.push(*id);
            }
        }
        revealed
    }

    /// Revealed cards that are still in a hidden zone. A revealed card stops
    /// being "revealed" information once it enters a public zone.
    pub fn revealed_hidden(&self) -> Vec<ObjectId> {
        self.revealed
            .iter()
            .copied()
            .filter(|id| {
                self.objects
                    .get(id)
                    .is_some_and(|object| matches!(object.zone, Zone::Library | Zone::Hand))
            })
            .collect()
    }

    fn prune_revealed(&mut self) {
        let objects = &self.objects;
        self.revealed.retain(|id| {
            objects
                .get(id)
                .is_some_and(|o| matches!(o.zone, Zone::Library | Zone::Hand))
        });
    }

    /// Untap one permanent (CR 701.26a). Returns whether anything changed.
    pub fn untap(&mut self, id: ObjectId) -> bool {
        match self.objects.get_mut(&id) {
            Some(object) if object.zone == Zone::Battlefield && object.tapped => {
                object.tapped = false;
                true
            }
            _ => false,
        }
    }

    /// Turn one permanent face down in the tapped position. Used by effects and
    /// tests; the untap step calls [`GameState::untap`] directly.
    pub fn tap(&mut self, id: ObjectId) -> bool {
        match self.objects.get_mut(&id) {
            Some(object) if object.zone == Zone::Battlefield && !object.tapped => {
                object.tapped = true;
                true
            }
            _ => false,
        }
    }

    /// Objects in one zone list owned by `player`, for callers that must not
    /// reach into the seat struct themselves.
    pub fn zone_list(&self, player: PlayerId, zone: Zone) -> Vec<ObjectId> {
        let seat = &self.players[player as usize];
        match zone {
            Zone::Library => seat.library.clone(),
            Zone::Hand => seat.hand.clone(),
            Zone::Graveyard => seat.graveyard.clone(),
            Zone::Exile => seat.exile.clone(),
            Zone::Command => seat.command_zone.clone(),
            Zone::Battlefield | Zone::Stack | Zone::Removed => Vec::new(),
        }
    }

    /// The zone of one object, or `None` when it no longer exists.
    pub fn zone_of(&self, id: ObjectId) -> Option<Zone> {
        self.objects.get(&id).map(|object: &Object| object.zone)
    }
}
