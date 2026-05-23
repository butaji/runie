//! Renderer crate with real taffy integration for layout and ratatui for rendering.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
};
use std::collections::HashMap;
use taffy::prelude::*;
use taffy::{NodeId, TaffyTree};

// ============================================================================
// Traits
// ============================================================================

/// Main renderer trait for mounting and rendering VNodes.
pub trait Renderer {
    fn mount(&mut self, node: VNode, area: Rect);
    fn render(&mut self, buffer: &mut ratatui::buffer::Buffer);
    fn measure(&mut self, node: &VNode, constraints: Constraints) -> Size;
}

// ============================================================================
// Types
// ============================================================================

/// Size in characters (width, height)
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

/// Layout constraints for measuring nodes.
#[derive(Debug, Clone, Default)]
pub struct Constraints {
    pub min_width: u16,
    pub max_width: u16,
    pub min_height: u16,
    pub max_height: u16,
}

/// CSS-like style properties for a VNode.
#[derive(Debug, Clone, Default)]
pub struct VStyle {
    pub flex_direction: FlexDirection,
    pub padding: u16,
    pub margin: u16,
    pub color: Option<String>,
    pub background_color: Option<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

/// Virtual node types.
#[derive(Clone)]
pub enum VNode {
    Element(VElement),
    Text(VText),
    Fragment(Vec<VNode>),
}

/// Virtual element (corresponds to JSX component/type).
#[derive(Clone)]
pub struct VElement {
    pub tag: String,
    pub props: VProps,
    pub children: Vec<VNode>,
}

/// Virtual text node.
#[derive(Clone)]
pub struct VText {
    pub content: String,
    pub style: VStyle,
}

/// Virtual props (style + event handlers).
/// Cannot derive Debug/Clone due to Fn fields.
pub struct VProps {
    pub style: VStyle,
    pub on_press: Option<Box<dyn Fn() + Send>>,
    pub on_change: Option<Box<dyn Fn(String) + Send>>,
}

impl Default for VProps {
    fn default() -> Self {
        Self {
            style: VStyle::default(),
            on_press: None,
            on_change: None,
        }
    }
}

impl Clone for VProps {
    fn clone(&self) -> Self {
        Self {
            style: self.style.clone(),
            on_press: None, // Can't clone closures
            on_change: None,
        }
    }
}

impl std::fmt::Debug for VNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VNode::Element(elem) => write!(f, "VNode::Element({})", elem.tag),
            VNode::Text(text) => write!(f, "VNode::Text({})", text.content),
            VNode::Fragment(children) => write!(f, "VNode::Fragment({} children)", children.len()),
        }
    }
}

impl std::fmt::Debug for VElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VElement({})", self.tag)
    }
}

impl std::fmt::Debug for VText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VText({})", self.content)
    }
}

// ============================================================================
// RatatuiRenderer
// ============================================================================

/// Ratatui renderer using taffy for layout computation.
pub struct RatatuiRenderer {
    taffy: TaffyTree,
    root: Option<VNode>,
    root_node_id: Option<NodeId>,
    node_ids: HashMap<String, NodeId>,
}

impl RatatuiRenderer {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            root: None,
            root_node_id: None,
            node_ids: HashMap::new(),
        }
    }

    fn build_taffy_tree(&mut self, node: &VNode, parent_id: NodeId) {
        match node {
            VNode::Element(elem) => {
                let flex_direction = match elem.props.style.flex_direction {
                    FlexDirection::Row => taffy::FlexDirection::Row,
                    FlexDirection::Column => taffy::FlexDirection::Column,
                };

                let padding = elem.props.style.padding as f32;
                let margin = elem.props.style.margin as f32;

                // Convert optional width/height to Dimension
                let width = elem.props.style.width
                    .map(|w| taffy::Dimension::Length(w as f32))
                    .unwrap_or(taffy::Dimension::Auto);
                let height = elem.props.style.height
                    .map(|h| taffy::Dimension::Length(h as f32))
                    .unwrap_or(taffy::Dimension::Auto);

                // Create child nodes first
                let child_ids: Vec<NodeId> = elem
                    .children
                    .iter()
                    .map(|child| match child {
                        VNode::Text(text) => {
                            let text_len = text.content.len() as f32;
                            let text_style = taffy::Style {
                                size: taffy::Size {
                                    width: taffy::Dimension::Length(text_len),
                                    height: taffy::Dimension::Length(1.0),
                                },
                                ..Default::default()
                            };
                            self.taffy.new_leaf(text_style).expect("Failed to create text node")
                        }
                        VNode::Element(_) => {
                            // Create placeholder; we'll build it recursively
                            self.taffy.new_leaf(taffy::Style::default()).expect("Failed to create element node")
                        }
                        VNode::Fragment(_) => {
                            self.taffy.new_leaf(taffy::Style::default()).expect("Failed to create fragment node")
                        }
                    })
                    .collect();

                let style = taffy::Style {
                    flex_direction,
                    size: taffy::Size { width, height },
                    padding: taffy::Rect {
                        left: LengthPercentage::from_length(padding),
                        right: LengthPercentage::from_length(padding),
                        top: LengthPercentage::from_length(padding),
                        bottom: LengthPercentage::from_length(padding),
                    },
                    margin: taffy::Rect {
                        left: LengthPercentageAuto::from_length(margin),
                        right: LengthPercentageAuto::from_length(margin),
                        top: LengthPercentageAuto::from_length(margin),
                        bottom: LengthPercentageAuto::from_length(margin),
                    },
                    ..Default::default()
                };

                let node_id = self
                    .taffy
                    .new_with_children(style, &child_ids)
                    .expect("Failed to create element node");

                self.node_ids.insert(elem.tag.clone(), node_id);
                let _ = self.taffy.add_child(parent_id, node_id);

                // Now recursively build children for element nodes
                for (i, child) in elem.children.iter().enumerate() {
                    if matches!(child, VNode::Element(_)) {
                        let child_node_id = child_ids[i];
                        self.build_taffy_tree(child, child_node_id);
                    }
                }
            }
            VNode::Text(text) => {
                let text_len = text.content.len() as f32;
                let style = taffy::Style {
                    size: taffy::Size {
                        width: taffy::Dimension::Length(text_len),
                        height: taffy::Dimension::Length(1.0),
                    },
                    ..Default::default()
                };
                let node_id = self.taffy.new_leaf(style).expect("Failed to create text node");
                let _ = self.taffy.add_child(parent_id, node_id);
            }
            VNode::Fragment(children) => {
                for child in children {
                    self.build_taffy_tree(child, parent_id);
                }
            }
        }
    }

    fn render_node_to_buffer(
        &self,
        node: &VNode,
        buffer: &mut Buffer,
        area: Rect,
        node_id: Option<NodeId>,
    ) {
        match node {
            VNode::Element(elem) => {
                // Draw background if set
                if let Some(bg) = &elem.props.style.background_color {
                    let color = parse_color(bg);
                    for y in area.y..area.y.saturating_add(area.height) {
                        for x in area.x..area.x.saturating_add(area.width) {
                            let cell = buffer.get_mut(x, y);
                            cell.set_style(Style::default().bg(color));
                        }
                    }
                }

                // Render children with their layouts from taffy
                if let Some(nid) = node_id {
                    let child_ids = self.taffy.children(nid).expect("Failed to get children");

                    for (i, child_id) in child_ids.iter().enumerate() {
                        if i < elem.children.len() {
                            if let Ok(layout) = self.taffy.layout(*child_id) {
                                // Use taffy layout location + parent area offset for correct positioning
                                let child_rect = Rect::new(
                                    area.x + layout.location.x as u16,
                                    area.y + layout.location.y as u16,
                                    layout.size.width as u16,
                                    layout.size.height as u16,
                                );
                                self.render_node_to_buffer(&elem.children[i], buffer, child_rect, Some(*child_id));
                            }
                        }
                    }
                }
            }
            VNode::Text(text) => {
                // Render text content
                let color = text
                    .style
                    .color
                    .as_ref()
                    .map(|c| parse_color(c))
                    .unwrap_or_default();

                let y = area.y;
                for (j, ch) in text.content.chars().enumerate() {
                    if j < area.width as usize {
                        let cell = buffer.get_mut(area.x + j as u16, y);
                        cell.set_char(ch).set_fg(color);
                    }
                }
            }
            VNode::Fragment(_children) => {
                // Fragments don't render themselves, children handle it
            }
        }
    }
}

impl Default for RatatuiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for RatatuiRenderer {
    fn mount(&mut self, node: VNode, area: Rect) {
        self.root = Some(node.clone());
        self.node_ids.clear();

        // Build new taffy tree
        let root_style = taffy::Style {
            size: taffy::Size {
                width: taffy::Dimension::Length(area.width as f32),
                height: taffy::Dimension::Length(area.height as f32),
            },
            ..Default::default()
        };

        let root_id = self.taffy.new_leaf(root_style).expect("Failed to create root");
        self.root_node_id = Some(root_id);

        // Build the VNode tree
        self.build_taffy_tree(&node, root_id);

        // Compute layout
        let available_space = taffy::Size {
            width: AvailableSpace::Definite(area.width as f32),
            height: AvailableSpace::Definite(area.height as f32),
        };
        self.taffy.compute_layout(root_id, available_space).ok();
    }

    fn render(&mut self, buffer: &mut ratatui::buffer::Buffer) {
        if let (Some(root), Some(root_id)) = (&self.root, self.root_node_id) {
            // Get root layout for full area
            if let Ok(layout) = self.taffy.layout(root_id) {
                let area = Rect::new(
                    0,
                    0,
                    layout.size.width as u16,
                    layout.size.height as u16,
                );
                self.render_node_to_buffer(root, buffer, area, Some(root_id));
            }
        }
    }

    fn measure(&mut self, node: &VNode, constraints: Constraints) -> Size {
        let flex_direction = match node.style().flex_direction {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
        };

        let style = taffy::Style {
            flex_direction,
            size: taffy::Size {
                width: taffy::Dimension::Length(constraints.max_width as f32),
                height: taffy::Dimension::Length(constraints.max_height as f32),
            },
            ..Default::default()
        };

        let node_id = self.taffy.new_leaf(style).ok();
        if let Some(id) = node_id {
            let available_space = taffy::Size {
                width: AvailableSpace::Definite(constraints.max_width as f32),
                height: AvailableSpace::Definite(constraints.max_height as f32),
            };
            self.taffy.compute_layout(id, available_space).ok();
            if let Ok(layout) = self.taffy.layout(id) {
                return Size {
                    width: layout.size.width as u16,
                    height: layout.size.height as u16,
                };
            }
        }

        Size::default()
    }
}

impl VNode {
    pub fn style(&self) -> VStyle {
        match self {
            VNode::Element(elem) => elem.props.style.clone(),
            VNode::Text(text) => text.style.clone(),
            VNode::Fragment(_) => VStyle::default(),
        }
    }

    pub fn tag(&self) -> &str {
        match self {
            VNode::Element(elem) => &elem.tag,
            VNode::Text(_) => "text",
            VNode::Fragment(_) => "fragment",
        }
    }
}

/// Parse hex color string to ratatui Color.
fn parse_color(hex: &str) -> ratatui::style::Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        ratatui::style::Color::Rgb(r, g, b)
    } else {
        ratatui::style::Color::Reset
    }
}

// ============================================================================
// VNode helpers for TSX-like construction
// ============================================================================

pub fn view(props: VProps, children: Vec<VNode>) -> VNode {
    VNode::Element(VElement {
        tag: "View".to_string(),
        props,
        children,
    })
}

pub fn text(content: &str, style: VStyle) -> VNode {
    VNode::Text(VText {
        content: content.to_string(),
        style,
    })
}

pub fn button(props: VProps, label: &str) -> VNode {
    VNode::Element(VElement {
        tag: "Button".to_string(),
        props,
        children: vec![text(label, VStyle::default())],
    })
}

pub fn style(flex_direction: Option<FlexDirection>) -> VStyle {
    let mut s = VStyle::default();
    if let Some(fd) = flex_direction {
        s.flex_direction = fd;
    }
    s
}
