// Authored by Kimi Code (AI coding agent) — task CHRON-020.
//! Minimal Phase 1 terrain: a surface kind plus walkability, nothing more.

use serde::{Deserialize, Serialize};

/// The exact set of Phase 1 surface variants (CHRON-020).
///
/// Deliberately excluded: biomes, ecology, elevation, moisture, resources,
/// and weighted movement costs. Walkability is the only movement semantic.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum TerrainKind {
    /// Open, walkable ground.
    Ground,
    /// Impassable water.
    Water,
    /// Impassable rock.
    Rock,
}

impl TerrainKind {
    /// Returns whether a person can stand on or cross this terrain.
    #[must_use]
    pub const fn is_walkable(self) -> bool {
        matches!(self, Self::Ground)
    }
}

#[cfg(test)]
mod tests {
    use super::TerrainKind;

    #[test]
    fn exactly_three_variants_with_fixed_walkability() {
        let variants = [TerrainKind::Ground, TerrainKind::Water, TerrainKind::Rock];
        let walkable: Vec<bool> = variants.iter().map(|kind| kind.is_walkable()).collect();
        assert_eq!(walkable, vec![true, false, false]);
    }

    #[test]
    fn serde_round_trip_per_variant() {
        for kind in [TerrainKind::Ground, TerrainKind::Water, TerrainKind::Rock] {
            let encoded = serde_json::to_string(&kind).expect("serialize terrain kind");
            let restored: TerrainKind =
                serde_json::from_str(&encoded).expect("deserialize terrain kind");
            assert_eq!(restored, kind);
        }
    }

    #[test]
    fn serde_rejects_unknown_variants() {
        assert!(serde_json::from_str::<TerrainKind>("\"Mountains\"").is_err());
        assert!(serde_json::from_str::<TerrainKind>("\"OpenPlains\"").is_err());
        assert!(serde_json::from_str::<TerrainKind>("\"Swamp\"").is_err());
    }
}
