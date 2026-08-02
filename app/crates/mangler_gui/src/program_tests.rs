use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use eframe::egui;

use super::{collect_selected_nodes_to_delete, detect_copy_paste, poll_preview_slot, PreviewPoll};

// === Event::Copy ===

#[test]
fn event_copy_triggers_copy() {
    let events = vec![egui::Event::Copy];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(copy);
    assert!(paste.is_none());
}

// === Event::Paste ===

#[test]
fn event_paste_returns_text() {
    let events = vec![egui::Event::Paste("hello".to_string())];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(!copy);
    assert_eq!(paste.as_deref(), Some("hello"));
}

#[test]
fn event_paste_empty_string_returns_some() {
    let events = vec![egui::Event::Paste(String::new())];
    let (_, paste) = detect_copy_paste(&events);
    assert_eq!(paste.as_deref(), Some(""));
}

#[test]
fn event_paste_with_node_data_returns_text() {
    // Simulates pasting clipboard data that was serialized by our copy handler.
    let text = "NODEMANGLER:{\"nodes\":[],\"connections\":[]}";
    let events = vec![egui::Event::Paste(text.to_string())];
    let (_, paste) = detect_copy_paste(&events);
    assert_eq!(paste.as_deref(), Some(text));
}

// === No relevant events ===

#[test]
fn no_events_triggers_nothing() {
    let events: Vec<egui::Event> = vec![];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(!copy);
    assert!(paste.is_none());
}

#[test]
fn unrelated_events_are_ignored() {
    let events = vec![egui::Event::Cut];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(!copy);
    assert!(paste.is_none());
}

// === Both copy and paste in one frame ===

#[test]
fn copy_and_paste_in_same_frame() {
    let events = vec![
        egui::Event::Copy,
        egui::Event::Paste("text".to_string()),
    ];
    let (copy, paste) = detect_copy_paste(&events);
    assert!(copy);
    assert_eq!(paste.as_deref(), Some("text"));
}

// === Last paste wins if multiple paste events in one frame ===

#[test]
fn multiple_paste_events_last_wins() {
    let events = vec![
        egui::Event::Paste("first".to_string()),
        egui::Event::Paste("second".to_string()),
    ];
    let (_, paste) = detect_copy_paste(&events);
    assert_eq!(paste.as_deref(), Some("second"));
}

// === collect_selected_nodes_to_delete ===

#[test]
fn delete_no_selection_returns_empty() {
    let mut selected = HashSet::new();
    let mut editing = Some("a".to_string());
    let result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    assert!(result.is_empty());
    // editing_node_id is untouched when nothing is selected
    assert_eq!(editing.as_deref(), Some("a"));
}

#[test]
fn delete_single_selected_node() {
    let mut selected = HashSet::from(["a".to_string()]);
    let mut editing = Some("a".to_string());
    let result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    assert_eq!(result, vec!["a".to_string()]);
    assert!(selected.is_empty());
    assert!(editing.is_none());
}

#[test]
fn delete_multiple_selected_nodes() {
    let mut selected = HashSet::from([
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
    ]);
    let mut editing = Some("b".to_string());
    let mut result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    result.sort();
    assert_eq!(result, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    assert!(selected.is_empty());
    assert!(editing.is_none());
}

#[test]
fn delete_selected_clears_editing_even_if_not_in_selection() {
    // editing_node_id might differ from selected set; it should still be cleared.
    let mut selected = HashSet::from(["a".to_string()]);
    let mut editing = Some("z".to_string());
    let result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    assert_eq!(result, vec!["a".to_string()]);
    assert!(editing.is_none());
}

#[test]
fn delete_selected_with_no_editing_node() {
    let mut selected = HashSet::from(["a".to_string(), "b".to_string()]);
    let mut editing: Option<String> = None;
    let mut result = collect_selected_nodes_to_delete(&mut selected, &mut editing);
    result.sort();
    assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
    assert!(selected.is_empty());
    assert!(editing.is_none());
}

// === poll_preview_slot (background library-image decode) ===

#[test]
fn preview_slot_empty_is_pending() {
    let slot: Mutex<Option<String>> = Mutex::new(None);
    assert_eq!(poll_preview_slot(&slot, 1, 1), PreviewPoll::Pending);
}

#[test]
fn preview_slot_current_generation_is_ready() {
    let slot = Mutex::new(Some("image".to_string()));
    assert_eq!(
        poll_preview_slot(&slot, 3, 3),
        PreviewPoll::Ready("image".to_string())
    );
}

#[test]
fn preview_slot_older_generation_is_stale() {
    // The user clicked another image while this one was still decoding.
    let slot = Mutex::new(Some("image".to_string()));
    assert_eq!(poll_preview_slot(&slot, 2, 5), PreviewPoll::Stale);
}

#[test]
fn preview_slot_ready_is_taken_out_of_the_slot() {
    // Promotion consumes the result: a second poll must not re-deliver it
    // (which would re-fit the view every frame).
    let slot = Mutex::new(Some(7));
    assert_eq!(poll_preview_slot(&slot, 1, 1), PreviewPoll::Ready(7));
    assert_eq!(poll_preview_slot(&slot, 1, 1), PreviewPoll::Pending);
}

#[test]
fn preview_slot_stale_result_is_discarded_not_left_behind() {
    let slot = Mutex::new(Some(7));
    assert_eq!(poll_preview_slot(&slot, 1, 2), PreviewPoll::Stale);
    assert!(slot.lock().unwrap().is_none());
}

#[test]
fn preview_slot_carries_decode_errors() {
    let slot: Mutex<Option<Result<u8, String>>> = Mutex::new(Some(Err("bad file".to_string())));
    assert_eq!(
        poll_preview_slot(&slot, 4, 4),
        PreviewPoll::Ready(Err("bad file".to_string()))
    );
}

#[test]
fn preview_slot_last_click_wins_across_two_decodes() {
    // Two decodes in flight, finishing out of order: only the newest one is
    // promoted, whichever lands first.
    let current = 9u64;
    let old: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let new: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    *new.lock().unwrap() = Some("new".to_string());
    assert_eq!(
        poll_preview_slot(&new, current, current),
        PreviewPoll::Ready("new".to_string())
    );

    *old.lock().unwrap() = Some("old".to_string());
    assert_eq!(poll_preview_slot(&old, current - 1, current), PreviewPoll::Stale);
}
