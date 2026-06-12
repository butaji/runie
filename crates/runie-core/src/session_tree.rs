//! Session tree — branching conversation history.

use crate::message::{ChatMessage, Role};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

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

/// Cached result of a filtered walk: (depth, path) pairs.
type FilterCache = Vec<(usize, Vec<usize>)>;

/// The session tree holds the root and current branch path.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SessionTree {
    pub root: TreeNode,
    pub current_branch: Vec<usize>,
    #[serde(skip)]
    pub node_index: HashMap<Vec<usize>, usize>,
    #[serde(skip)]
    pub index_version: u64,
    #[serde(skip)]
    built_version: u64,
    #[serde(skip)]
    cached_filter: RefCell<Option<(SessionTreeFilter, u64, FilterCache)>>,
}

impl PartialEq for SessionTree {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.current_branch == other.current_branch
    }
}

impl SessionTree {
    pub fn new(root_message: ChatMessage) -> Self {
        Self {
            root: TreeNode::new(root_message),
            current_branch: Vec::new(),
            node_index: HashMap::new(),
            index_version: 1,
            built_version: 0,
            cached_filter: RefCell::new(None),
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
        tree.rebuild_index();
        tree.built_version = tree.index_version;
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
        self.invalidate_index();
        Some(path)
    }

    /// Navigate to a path in the tree using O(1) index lookup.
    pub fn navigate_to(&mut self, path: &[usize]) {
        self.ensure_index();
        if self.node_index.contains_key(path) {
            self.current_branch = path.to_vec();
        }
    }

    /// Find the node corresponding to a flat message index (0-based).
    fn node_at_message_index(&mut self, index: usize) -> Option<&mut TreeNode> {
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

    /// Walk the tree collecting (depth, node, path) tuples.
    fn walk_with_paths(&self) -> Vec<(usize, &TreeNode, Vec<usize>)> {
        let mut result = Vec::new();
        Self::walk_node_with_paths(&self.root, 0, &[], &mut result);
        result
    }

    /// Find the path to the node whose message has the given id.
    pub fn find_path_by_id(&self, id: &str) -> Option<Vec<usize>> {
        self.walk_with_paths()
            .into_iter()
            .find(|(_, node, _)| node.message.id == id)
            .map(|(_, _, path)| path)
    }

    fn walk_node_with_paths<'a>(
        node: &'a TreeNode,
        depth: usize,
        path: &[usize],
        result: &mut Vec<(usize, &'a TreeNode, Vec<usize>)>,
    ) {
        result.push((depth, node, path.to_vec()));
        for (i, child) in node.children.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(i);
            Self::walk_node_with_paths(child, depth + 1, &child_path, result);
        }
    }

    /// Resolve a path to a node reference.
    fn node_at_path(&self, path: &[usize]) -> Option<&TreeNode> {
        let mut node = &self.root;
        for &idx in path {
            node = node.children.get(idx)?;
        }
        Some(node)
    }

    /// Collect visible nodes given a filter, with caching.
    pub fn filtered_walk(&self, filter: SessionTreeFilter) -> Vec<(usize, &TreeNode)> {
        // Try cache first
        if let Ok(cache) = self.cached_filter.try_borrow() {
            if let Some((cached_filter, cached_version, cached_paths)) = cache.as_ref() {
                if *cached_filter == filter && *cached_version == self.index_version {
                    return cached_paths
                        .iter()
                        .filter_map(|(depth, path)| {
                            self.node_at_path(path).map(|node| (*depth, node))
                        })
                        .collect();
                }
            }
        }

        // Compute fresh result
        let all = self.walk_with_paths();
        let result: Vec<_> = all
            .into_iter()
            .filter(|(_, n, _)| filter.passes(&n.message, n.label.as_deref()))
            .collect();

        // Extract paths for cache and return references
        let paths: Vec<_> = result.iter().map(|(d, _, p)| (*d, p.clone())).collect();
        let output: Vec<_> = result.into_iter().map(|(d, n, _)| (d, n)).collect();

        // Store in cache
        if let Ok(mut cache) = self.cached_filter.try_borrow_mut() {
            *cache = Some((filter, self.index_version, paths));
        }

        output
    }

    /// Rebuild the node index from scratch.
    fn rebuild_index(&mut self) {
        self.node_index.clear();
        let mut paths = Vec::new();
        Self::collect_paths(&self.root, &[], &mut paths);
        for (i, path) in paths.into_iter().enumerate() {
            self.node_index.insert(path, i);
        }
    }

    fn collect_paths(node: &TreeNode, path: &[usize], paths: &mut Vec<Vec<usize>>) {
        paths.push(path.to_vec());
        for (i, child) in node.children.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(i);
            Self::collect_paths(child, &child_path, paths);
        }
    }

    /// Ensure the index is up to date; lazily rebuild if invalidated.
    fn ensure_index(&mut self) {
        if self.built_version != self.index_version {
            self.rebuild_index();
            self.built_version = self.index_version;
        }
    }

    /// Invalidate the index and filter cache after tree mutation.
    fn invalidate_index(&mut self) {
        self.index_version = self.index_version.wrapping_add(1);
        self.built_version = self.index_version.wrapping_sub(1); // force rebuild
        self.node_index.clear();
        if let Ok(mut cache) = self.cached_filter.try_borrow_mut() {
            *cache = None;
        }
    }
}
