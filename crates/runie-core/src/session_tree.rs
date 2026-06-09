//! Session tree — branching conversation history.

use crate::message::{ChatMessage, Role};
use serde::{Deserialize, Serialize};

/// A node in the session tree.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TreeNode {
    pub message: ChatMessage,
    pub children: Vec<TreeNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl TreeNode {
    pub fn new(message: ChatMessage) -> Self {
        Self {
            message,
            children: Vec::new(),
            label: None,
        }
    }

    /// Insert a child at the given index.
    pub fn insert_child(&mut self, index: usize, node: TreeNode) {
        self.children.insert(index, node);
    }

    /// Append a child.
    pub fn add_child(&mut self, node: TreeNode) {
        self.children.push(node);
    }

    /// Total number of nodes in this subtree.
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }
}

/// Tree filter for the session tree dialog.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionTreeFilter {
    #[default]
    All,
    NoTools,
    UserOnly,
    LabeledOnly,
}

impl SessionTreeFilter {
    pub fn cycle(self) -> Self {
        match self {
            Self::All => Self::NoTools,
            Self::NoTools => Self::UserOnly,
            Self::UserOnly => Self::LabeledOnly,
            Self::LabeledOnly => Self::All,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::NoTools => "no-tools",
            Self::UserOnly => "user-only",
            Self::LabeledOnly => "labeled-only",
        }
    }

    /// Returns true if a message passes this filter.
    pub fn passes(&self, msg: &ChatMessage, label: Option<&str>) -> bool {
        match self {
            Self::All => true,
            Self::NoTools => msg.role != Role::Tool,
            Self::UserOnly => msg.role == Role::User,
            Self::LabeledOnly => label.is_some(),
        }
    }
}

/// The session tree holds the root and current branch path.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionTree {
    pub root: TreeNode,
    pub current_branch: Vec<usize>,
}

impl SessionTree {
    pub fn new(root_message: ChatMessage) -> Self {
        Self {
            root: TreeNode::new(root_message),
            current_branch: Vec::new(),
        }
    }

    /// Build a session tree from a flat list of messages.
    /// Each message becomes a child of the previous, forming a linear tree.
    pub fn from_messages(messages: &[ChatMessage]) -> Self {
        if messages.is_empty() {
            return Self::default();
        }
        let mut tree = Self::new(messages[0].clone());
        let mut current = &mut tree.root;
        for msg in &messages[1..] {
            current.add_child(TreeNode::new(msg.clone()));
            // Move to the last child (linear tree)
            let idx = current.children.len() - 1;
            current = &mut current.children[idx];
            tree.current_branch.push(idx);
        }
        tree
    }

    /// Get the node at the end of the current branch.
    pub fn current_node(&self) -> Option<&TreeNode> {
        let mut node = &self.root;
        for &idx in &self.current_branch {
            node = node.children.get(idx)?;
        }
        Some(node)
    }

    /// Get a mutable reference to the node at the end of the current branch.
    pub fn current_node_mut(&mut self) -> Option<&mut TreeNode> {
        let mut node = &mut self.root;
        for &idx in &self.current_branch {
            node = node.children.get_mut(idx)?;
        }
        Some(node)
    }

    /// Fork a new branch from the message at `message_index`.
    /// The new branch starts as a child of that message node.
    /// Returns the path to the new branch.
    pub fn fork_at(&mut self, message_index: usize) -> Option<Vec<usize>> {
        let target = self.node_at_message_index(message_index)?;
        // Create a placeholder child to mark the fork point
        let placeholder = ChatMessage {
            role: Role::System,
            content: "[fork point]".to_string(),
            timestamp: crate::message::now(),
            id: format!("fork.{}", message_index),
            ..Default::default()
        };
        target.add_child(TreeNode::new(placeholder));
        let child_idx = target.children.len() - 1;
        let mut path = self.path_to_message_index(message_index)?;
        path.push(child_idx);
        Some(path)
    }

    /// Navigate to a path in the tree.
    pub fn navigate_to(&mut self, path: &[usize]) {
        // Validate the path exists
        let mut node = &self.root;
        for &idx in path {
            if let Some(child) = node.children.get(idx) {
                node = child;
            } else {
                return;
            }
        }
        self.current_branch = path.to_vec();
    }

    /// Find the node corresponding to a flat message index (0-based).
    fn node_at_message_index(&mut self, index: usize) -> Option<&mut TreeNode> {
        // Flatten the current branch to find the message at the given index
        let path = self.path_to_message_index(index)?;
        let mut node = &mut self.root;
        for &idx in &path {
            node = node.children.get_mut(idx)?;
        }
        Some(node)
    }

    /// Build the path to the message at a given flat index along the current branch.
    fn path_to_message_index(&self, index: usize) -> Option<Vec<usize>> {
        if index == 0 {
            return Some(Vec::new());
        }
        let mut path = Vec::new();
        let mut node = &self.root;
        for _ in 1..=index {
            if node.children.is_empty() {
                return None;
            }
            // Follow the current branch if available, otherwise first child
            let idx = if !path.is_empty() || !self.current_branch.is_empty() {
                let branch_idx = path.len();
                if branch_idx < self.current_branch.len() {
                    self.current_branch[branch_idx]
                } else {
                    0
                }
            } else {
                self.current_branch.first().copied().unwrap_or(0)
            };
            let actual_idx = idx.min(node.children.len() - 1);
            path.push(actual_idx);
            node = &node.children[actual_idx];
        }
        Some(path)
    }

    /// Collect all nodes in the tree in pre-order, returning (depth, node) pairs.
    pub fn walk(&self) -> Vec<(usize, &TreeNode)> {
        let mut result = Vec::new();
        Self::walk_node(&self.root, 0, &mut result);
        result
    }

    fn walk_node<'a>(node: &'a TreeNode, depth: usize, result: &mut Vec<(usize, &'a TreeNode)>) {
        result.push((depth, node));
        for child in &node.children {
            Self::walk_node(child, depth + 1, result);
        }
    }

    /// Collect visible nodes given a filter.
    pub fn filtered_walk(&self, filter: SessionTreeFilter) -> Vec<(usize, &TreeNode)> {
        let all = self.walk();
        all.into_iter()
            .filter(|(_, n)| filter.passes(&n.message, n.label.as_deref()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: Role, content: &str) -> ChatMessage {
        ChatMessage {
            role,
            content: content.into(),
            timestamp: 0.0,
            id: "test".into(),
            ..Default::default()
        }
    }

    #[test]
    fn tree_from_messages_linear() {
        let messages = vec![
            msg(Role::User, "hello"),
            msg(Role::Assistant, "hi"),
            msg(Role::User, "how are you"),
        ];
        let tree = SessionTree::from_messages(&messages);
        assert_eq!(tree.root.message.content, "hello");
        assert_eq!(tree.current_branch, vec![0, 0]);
        assert_eq!(tree.root.count(), 3);
    }

    #[test]
    fn fork_creates_branch() {
        let messages = vec![
            msg(Role::User, "hello"),
            msg(Role::Assistant, "hi"),
            msg(Role::User, "how are you"),
        ];
        let mut tree = SessionTree::from_messages(&messages);
        let path = tree.fork_at(1).expect("fork should succeed");
        assert_eq!(path.len(), 2);
        let node = tree.current_node();
        assert!(node.is_some());
    }

    #[test]
    fn filter_excludes_tools() {
        let mut tree = SessionTree::from_messages(&[
            msg(Role::User, "hello"),
            msg(Role::Tool, "output"),
            msg(Role::Assistant, "hi"),
        ]);
        // Add a labeled node
        tree.root.children[0].label = Some("important".into());

        let all = tree.filtered_walk(SessionTreeFilter::All);
        assert_eq!(all.len(), 3);

        let no_tools = tree.filtered_walk(SessionTreeFilter::NoTools);
        assert_eq!(no_tools.len(), 2);

        let user_only = tree.filtered_walk(SessionTreeFilter::UserOnly);
        assert_eq!(user_only.len(), 1);

        let labeled = tree.filtered_walk(SessionTreeFilter::LabeledOnly);
        assert_eq!(labeled.len(), 1);
    }

    #[test]
    fn filter_cycle_rotates() {
        assert_eq!(SessionTreeFilter::All.cycle(), SessionTreeFilter::NoTools);
        assert_eq!(SessionTreeFilter::NoTools.cycle(), SessionTreeFilter::UserOnly);
        assert_eq!(SessionTreeFilter::UserOnly.cycle(), SessionTreeFilter::LabeledOnly);
        assert_eq!(SessionTreeFilter::LabeledOnly.cycle(), SessionTreeFilter::All);
    }

    #[test]
    fn clone_duplicates_position() {
        let messages = vec![
            msg(Role::User, "hello"),
            msg(Role::Assistant, "hi"),
            msg(Role::User, "how are you"),
        ];
        let tree = SessionTree::from_messages(&messages);
        let cloned = tree.clone();
        assert_eq!(cloned.root.message.content, "hello");
        assert_eq!(cloned.current_branch, tree.current_branch);
    }
}
