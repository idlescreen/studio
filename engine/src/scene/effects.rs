// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Resolving an effect name against the daemon's scene vocabulary.

use crate::error::RenderError;
use idlescreen::ascii::effects::EffectKind;

/// Every scene name the daemon ships, in the daemon's own order.
///
/// The list is read from the daemon rather than kept here: a local copy
/// would drift the moment a scene is added, and a name the CLI accepts but
/// the daemon does not is a worse failure than an unknown name.
pub fn effect_names() -> Vec<&'static str> {
    EffectKind::ALL.iter().map(|e| e.name()).collect()
}

/// The scene names, as a comma-separated list for error messages.
pub fn effect_names_joined() -> String {
    effect_names().join(", ")
}

/// Resolve a user-supplied effect name to a scene.
///
/// The daemon accepts a handful of aliases (`megaman` for `mmx`, `shower`
/// for `rain`, …), so the resolution goes through the daemon rather than
/// matching a local list.
pub fn resolve_effect(name: &str) -> Result<EffectKind, RenderError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(RenderError::Job("effect name is empty".into()));
    }
    EffectKind::parse(trimmed).ok_or_else(|| {
        RenderError::Job(format!(
            "unknown effect '{trimmed}'. The daemon ships {} scenes: {}",
            EffectKind::ALL.len(),
            effect_names_joined()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_daemon_scene_resolves() {
        let names = effect_names();
        assert_eq!(names.len(), 53);
        for name in names {
            assert!(resolve_effect(name).is_ok(), "{name} did not resolve");
        }
    }

    #[test]
    fn the_list_has_no_duplicates() {
        let mut names = effect_names();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before);
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!(resolve_effect("megaman").ok(), Some(EffectKind::Mmx));
        assert_eq!(resolve_effect("shower").ok(), Some(EffectKind::Rain));
        assert_eq!(resolve_effect("zsaber").ok(), Some(EffectKind::Zero));
    }

    #[test]
    fn an_unknown_name_lists_the_valid_ones() {
        let err = resolve_effect("nope").expect_err("unknown effect");
        let message = err.to_string();
        assert!(message.contains("nope"), "{message}");
        assert!(message.contains("aurora"), "{message}");
    }

    #[test]
    fn an_empty_name_is_rejected() {
        assert!(resolve_effect("   ").is_err());
    }
}
