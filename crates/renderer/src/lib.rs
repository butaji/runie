//! Renderer crate with real taffy integration for layout and ratatui for rendering.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
};
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
            on_press: None,
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
    root_id: Option<NodeId>,
}

impl RatatuiRenderer {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            root: None,
            root_id: None,
        }
    }

    /// Build taffy tree from VNode recursively.
    /// Returns the NodeId for this subtree.
    fn build_taffy_tree(&mut self, node: &VNode) -> Option<NodeId> {
        match node {
            VNode::Element(elem) => {
                let flex_direction = match elem.props.style.flex_direction {
                    FlexDirection::Row => taffy::FlexDirection::Row,
                    FlexDirection::Column => taffy::FlexDirection::Column,
                };

                let padding = elem.props.style.padding as f32;
                let margin = elem.props.style.margin as f32;

                let width = elem.props.style.width
                    .map(|w| taffy::Dimension::Length(w as f32))
                    .unwrap_or(taffy::Dimension::Auto);
                let height = elem.props.style.height
                    .map(|h| taffy::Dimension::Length(h as f32))
                    .unwrap_or(taffy::Dimension::Auto);

                // Recursively build children
                let child_ids: Vec<NodeId> = elem
                    .children
                    .iter()
                    .filter_map(|child| self.build_taffy_tree(child))
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

                let node_id = if child_ids.is_empty() {
                    self.taffy.new_leaf(style).ok()?
                } else {
                    self.taffy.new_with_children(style, &child_ids).ok()?
                };

                Some(node_id)
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
                self.taffy.new_leaf(style).ok()
            }
            VNode::Fragment(children) => {
                // Fragments inline their children; return first child's id
                // (simplified: fragments are transparent in layout)
                children.iter().filter_map(|c| self.build_taffy_tree(c)).next()
            }
        }
    }

    /// Render a node and its children recursively.
    /// `node_id` is the taffy node corresponding to this VNode.
    /// `area` is the screen rect allocated to this node.
    fn render_node(
        &self,
        node: &VNode,
        buffer: &mut Buffer,
        area: Rect,
        node_id: NodeId,
    ) {
        match node {
            VNode::Element(elem) => {
                // Draw View border
                if elem.tag == "View" {
                    self.draw_box(buffer, area);
                }

                // Button renders itself specially
                if elem.tag == "Button" {
                    self.render_button(elem, buffer, area);
                    return;
                }

                // For other elements, render children using taffy layouts
                if let Ok(child_ids) = self.taffy.children(node_id) {
                    for (i, child) in elem.children.iter().enumerate() {
                        if i < child_ids.len() {
                            if let Ok(layout) = self.taffy.layout(child_ids[i]) {
                                let child_rect = Rect::new(
                                    area.x + layout.location.x as u16,
                                    area.y + layout.location.y as u16,
                                    layout.size.width as u16,
                                    layout.size.height as u16,
                                );
                                self.render_node(child, buffer, child_rect, child_ids[i]);
                            }
                        }
                    }
                }
            }
            VNode::Text(text) => {
                let color = text
                    .style
                    .color
                    .as_ref()
                    .map(|c| parse_color(c))
                    .unwrap_or(ratatui::style::Color::Reset);

                let y = area.y;
                let content = if text.content.chars().count() > area.width as usize {
                    text.content.chars().take(area.width as usize - 1).collect::<String>() + "…"
                } else {
                    text.content.clone()
                };

                for (j, ch) in content.chars().enumerate() {
                    if j < area.width as usize {
                        let cell = buffer.get_mut(area.x + j as u16, y);
                        cell.set_char(ch).set_fg(color);
                    }
                }
            }
            VNode::Fragment(children) => {
                // Fragments render children at same position
                if let Ok(child_ids) = self.taffy.children(node_id) {
                    for (i, child) in children.iter().enumerate() {
                        if i < child_ids.len() {
                            if let Ok(layout) = self.taffy.layout(child_ids[i]) {
                                let child_rect = Rect::new(
                                    area.x + layout.location.x as u16,
                                    area.y + layout.location.y as u16,
                                    layout.size.width as u16,
                                    layout.size.height as u16,
                                );
                                self.render_node(child, buffer, child_rect, child_ids[i]);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Draw a border box using box-drawing characters.
    fn draw_box(&self, buffer: &mut Buffer, area: Rect) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let style = Style::default();
        let w = area.width as usize;
        let h = area.height as usize;
        let x0 = area.x as usize;
        let y0 = area.y as usize;
        let x1 = x0 + w - 1;
        let y1 = y0 + h - 1;

        // Corners
        buffer.get_mut(x0 as u16, y0 as u16).set_char('┌').set_style(style);
        buffer.get_mut(x1 as u16, y0 as u16).set_char('┐').set_style(style);
        buffer.get_mut(x0 as u16, y1 as u16).set_char('└').set_style(style);
        buffer.get_mut(x1 as u16, y1 as u16).set_char('┘').set_style(style);

        // Horizontal lines
        for x in (x0 + 1)..x1 {
            buffer.get_mut(x as u16, y0 as u16).set_char('─').set_style(style);
            buffer.get_mut(x as u16, y1 as u16).set_char('─').set_style(style);
        }

        // Vertical lines
        for y in (y0 + 1)..y1 {
            buffer.get_mut(x0 as u16, y as u16).set_char('│').set_style(style);
            buffer.get_mut(x1 as u16, y as u16).set_char('│').set_style(style);
        }
    }

    /// Draw button with brackets.
    fn render_button(&self, elem: &VElement, buffer: &mut Buffer, area: Rect) {
        let label: String = elem
            .children
            .iter()
            .filter_map(|c| {
                if let VNode::Text(t) = c {
                    Some(t.content.clone())
                } else {
                    None
                }
            })
            .collect();

        let color = ratatui::style::Color::Yellow;
        let label_len = label.len() as u16;
        let total_width = label_len + 4; // [ + space + label + space + ]
        let start_x = if area.width > total_width {
            area.x + (area.width - total_width) / 2
        } else {
            area.x
        };
        let y = area.y + area.height / 2;

        // Draw brackets
        buffer.get_mut(start_x, y).set_char('[').set_fg(color);
        buffer.get_mut(start_x + label_len + 3, y).set_char(']').set_fg(color);

        // Draw label
        for (i, ch) in label.chars().enumerate() {
            if (start_x + 2 + i as u16) < area.x + area.width {
                buffer.get_mut(start_x + 2 + i as u16, y).set_char(ch).set_fg(color);
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

        // Build taffy tree
        let root_id = self.build_taffy_tree(&node);

        if let Some(root_id) = root_id {
            // Set root size to fill the area
            let _ = self.taffy.set_style(
                root_id,
                taffy::Style {
                    size: taffy::Size {
                        width: taffy::Dimension::Length(area.width as f32),
                        height: taffy::Dimension::Length(area.height as f32),
                    },
                    ..Default::default()
                },
            );

            let available_space = taffy::Size {
                width: AvailableSpace::Definite(area.width as f32),
                height: AvailableSpace::Definite(area.height as f32),
            };
            self.taffy.compute_layout(root_id, available_space).ok();
            self.root_id = Some(root_id);
        }
    }

    fn render(&mut self, buffer: &mut ratatui::buffer::Buffer) {
        if let (Some(root), Some(root_id)) = (&self.root, self.root_id) {
            let area = *buffer.area();
            self.render_node(root, buffer, area, root_id);
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

        if let Some(node_id) = self.taffy.new_leaf(style).ok() {
            let available_space = taffy::Size {
                width: AvailableSpace::Definite(constraints.max_width as f32),
                height: AvailableSpace::Definite(constraints.max_height as f32),
            };
            self.taffy.compute_layout(node_id, available_space).ok();
            if let Ok(layout) = self.taffy.layout(node_id) {
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
