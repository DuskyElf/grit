use anyhow::Result;

use crate::provider::{DiffPatch, PlaylistSnapshot, Track, TrackChange};
use std::collections::HashMap;

pub fn diff(old: &PlaylistSnapshot, new: &PlaylistSnapshot) -> DiffPatch {
    let mut changes = Vec::new();

    //idx_map : track_id -> (index, &Track)
    let old_map: HashMap<String, (usize, &Track)> = old
        .tracks
        .iter()
        .enumerate()
        .map(|(idx, track)| (track.id.clone(), (idx, track)))
        .collect();

    let new_map: HashMap<String, (usize, &Track)> = new
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.clone(), (i, t)))
        .collect();

    // Find removed tracks
    for (id, (old_idx, track)) in &old_map {
        if !new_map.contains_key(id) {
            changes.push(TrackChange::Removed {
                track: (*track).clone(),
                index: *old_idx,
            });
        }
    }
    //Find added tracks
    for (id, (new_index, track)) in &new_map {
        if !old_map.contains_key(id) {
            changes.push(TrackChange::Added {
                track: (*track).clone(),
                index: *new_index,
            });
        }
    }
    //Find moved tracks
    for (id, (new_index, track)) in &new_map {
        if let Some((old_index, _)) = old_map.get(id) {
            if old_index != new_index {
                changes.push(TrackChange::Moved {
                    track: (*track).clone(),
                    from: *old_index,
                    to: *new_index,
                });
            }
        }
    }

    DiffPatch { changes }
}

pub fn apply_patch(snapshot: &mut PlaylistSnapshot, patch: &DiffPatch) -> Result<()> {
    // Process changes in correct order:
    // 1. Removals (from highest index to lowest to avoid shifting issues)
    // 2. Additions
    // 3. Moves

    let mut removals = Vec::new();
    let mut additions = Vec::new();
    let mut moves = Vec::new();

    for change in &patch.changes {
        match change {
            TrackChange::Removed { index, .. } => removals.push((*index, change)),
            TrackChange::Added { .. } => additions.push(change),
            TrackChange::Moved { .. } => moves.push(change),
        }
    }

    // Sort removals by index (highest first to avoid shifting)
    removals.sort_by(|a, b| b.0.cmp(&a.0));

    //remove
    for (_, change) in removals {
        if let TrackChange::Removed { index, .. } = change {
            if *index < snapshot.tracks.len() {
                snapshot.tracks.remove(*index);
            }
        }
    }

    //add
    for change in additions {
        if let TrackChange::Added { track, index } = change {
            if *index <= snapshot.tracks.len() {
                snapshot.tracks.insert(*index, track.clone());
            } else {
                snapshot.tracks.push(track.clone());
            }
        }
    }

    //move
    for change in moves {
        if let TrackChange::Moved { from, to, .. } = change {
            if *from < snapshot.tracks.len() && *to < snapshot.tracks.len() {
                let track = snapshot.tracks.remove(*from);
                snapshot.tracks.insert(*to, track);
            }
        }
    }

    Ok(())
}
