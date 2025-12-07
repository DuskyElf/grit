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
