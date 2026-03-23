use std::collections::HashSet;

use eframe::egui;

use super::{collect_selected_nodes_to_delete, detect_copy_paste};

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
