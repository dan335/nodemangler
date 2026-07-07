use serde::{Deserialize, Serialize};

/// The content a panel can display. Every panel in the tree shows exactly one
/// of these, switchable at runtime via the panel's corner menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelKind {
    Graph,
    Preview2D,
    Preview3D,
    NodeList,
    Libraries,
    Settings,
}

impl PanelKind {
    /// All panel kinds, in the order they appear in the panel-kind menu.
    /// Libraries sits right after NodeList — the two share the left column in
    /// the default layout.
    pub const ALL: [PanelKind; 6] = [
        PanelKind::Graph,
        PanelKind::Preview2D,
        PanelKind::Preview3D,
        PanelKind::NodeList,
        PanelKind::Libraries,
        PanelKind::Settings,
    ];

    /// Human-readable name shown in the panel-kind menu.
    pub fn label(&self) -> &'static str {
        match self {
            PanelKind::Graph => "Graph",
            PanelKind::Preview2D => "2D Preview",
            PanelKind::Preview3D => "3D Preview",
            PanelKind::NodeList => "Node List",
            PanelKind::Libraries => "Libraries",
            PanelKind::Settings => "Settings",
        }
    }

    /// Phosphor icon glyph shown on the panel's corner button.
    pub fn icon(&self) -> &'static str {
        match self {
            PanelKind::Graph => egui_phosphor::regular::GRAPH,
            PanelKind::Preview2D => egui_phosphor::regular::IMAGE,
            PanelKind::Preview3D => egui_phosphor::regular::CUBE,
            PanelKind::NodeList => egui_phosphor::regular::LIST,
            PanelKind::Libraries => egui_phosphor::regular::BOOKS,
            PanelKind::Settings => egui_phosphor::regular::SLIDERS,
        }
    }
}
