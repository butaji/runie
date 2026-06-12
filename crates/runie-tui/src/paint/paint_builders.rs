//! Paint builders - convenience constructors for paint nodes.

use super::{Border, Col, Node, Pad, Row, StyleRef, Text};

pub fn row(children: Vec<Node>) -> Row {
    Row { children, gap: 0, bg: StyleRef::BgBase, fill_trailing: false }
}

pub fn col(children: Vec<Node>) -> Col {
    Col { children, gap: 0, bg: StyleRef::BgBase }
}

impl Row {
    pub fn gap(mut self, n: u16) -> Self { self.gap = n; self }
    pub fn bg(mut self, s: StyleRef) -> Self { self.bg = s; self }
    pub fn fill_trailing(mut self) -> Self { self.fill_trailing = true; self }
    pub fn child(mut self, n: Node) -> Self { self.children.push(n); self }
    pub fn children(mut self, mut cs: Vec<Node>) -> Self {
        self.children.append(&mut cs);
        self
    }
    pub fn build(self) -> Node { Node::Row(self) }
}

impl Col {
    pub fn gap(mut self, n: u16) -> Self { self.gap = n; self }
    pub fn bg(mut self, s: StyleRef) -> Self { self.bg = s; self }
    pub fn child(mut self, n: Node) -> Self { self.children.push(n); self }
    pub fn build(self) -> Node { Node::Col(self) }
}

pub fn border(child: Node) -> Border {
    Border { child: Some(Box::new(child)), title: None, style: StyleRef::Dim }
}

impl Border {
    pub fn title<T: Into<String>>(mut self, t: T) -> Self {
        self.title = Some(Text::new(t));
        self
    }
    pub fn style(mut self, s: StyleRef) -> Self { self.style = s; self }
    pub fn build(self) -> Node { Node::Border(self) }
}

pub fn pad(child: Node) -> Pad {
    Pad { child: Box::new(child), top: 0, right: 0, bottom: 0, left: 0 }
}

impl Pad {
    pub fn all(mut self, n: u16) -> Self {
        self.top = n; self.right = n; self.bottom = n; self.left = n;
        self
    }
    pub fn xy(mut self, x: u16, y: u16) -> Self {
        self.left = x; self.right = x; self.top = y; self.bottom = y;
        self
    }
    pub fn build(self) -> Node { Node::Pad(self) }
}
