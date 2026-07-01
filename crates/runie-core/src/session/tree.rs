//! Session tree — branching conversation history using an arena-backed tree.
//!
//! Uses `indextree` for stable `NodeId` handles and deterministic traversal.

use crate::message::{ChatMessage, Role};
use indextree::{Arena, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// A node in the session tree — holds the data, not the tree structure.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TreeNodeData {
    pub message: ChatMessage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl TreeNodeData {
    pub fn new(message: ChatMessage) -> Self {
        Self {
            message,
            label: None,
        }
    }
}

/// Tree filter for the session tree dialog.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, strum::Display)]
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

    /// String representation for display (kebab-case).
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

/// Cached result of a filtered walk: (depth, NodeId) pairs.
type FilterCache = Vec<(usize, NodeId)>;

/// A node in the serialized tree — stores message data and parent relationship.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedNode {
    /// Unique message ID (used for parent references).
    pub id: String,
    /// Node data (message + optional label).
    pub data: TreeNodeData,
}

/// Serialized form of SessionTree — stores edges and branch path.
/// The arena is rebuilt from edges on deserialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionTreeSnapshot {
    /// Current branch path as message IDs (stable across serialization).
    #[serde(default)]
    pub current_branch: Vec<String>,
    /// Root node message ID.
    #[serde(default)]
    pub root_id: String,
    /// All nodes: each stores its parent message ID for edge reconstruction.
    #[serde(default)]
    pub nodes: Vec<SerializedNode>,
    /// Parent-child edges: (parent_id, child_id) pairs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<(String, String)>,
}

impl Default for SessionTreeSnapshot {
    fn default() -> Self {
        Self {
            current_branch: Vec::new(),
            root_id: "root".to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

/// The session tree holds the arena and current branch path.
pub struct SessionTree {
    /// Arena backing the tree structure.
    arena: Arena<TreeNodeData>,
    /// Root node ID.
    root_id: Option<NodeId>,
    /// Current branch path as node IDs.
    current_branch: Vec<NodeId>,
    /// Index from message ID to NodeId for O(1) lookup.
    id_index: HashMap<String, NodeId>,
    /// Cached filtered walk results.
    cached_filter: Mutex<Option<(SessionTreeFilter, FilterCache)>>,
}

// ─── Serialization ─────────────────────────────────────────────────────────

impl Serialize for SessionTree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_snapshot().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SessionTree {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let snapshot = SessionTreeSnapshot::deserialize(deserializer)?;
        Self::from_snapshot(&snapshot)
            .ok_or_else(|| serde::de::Error::custom("invalid session tree snapshot"))
    }
}

impl Clone for SessionTree {
    fn clone(&self) -> Self {
        // Serialize to snapshot and deserialize to get a proper deep clone.
        let snapshot = self.to_snapshot();
        Self::from_snapshot(&snapshot)
            .expect("session tree clone failed — snapshot should always be valid")
    }
}

impl std::fmt::Debug for SessionTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionTree")
            .field("arena_len", &self.arena.iter().count())
            .field("root_id", &self.root_id)
            .field("current_branch", &self.current_branch.len())
            .field("id_index", &self.id_index.len())
            .finish()
    }
}

impl Default for SessionTree {
    fn default() -> Self {
        let mut arena = Arena::new();
        let root_data = TreeNodeData::new(ChatMessage {
            role: Role::System,
            id: "root".into(),
            parts: vec![crate::message::Part::Text {
                content: "[session root]".into(),
            }],
            ..Default::default()
        });
        let root_id = arena.new_node(root_data);
        Self {
            arena,
            root_id: Some(root_id),
            current_branch: Vec::new(),
            id_index: HashMap::new(),
            cached_filter: Mutex::new(None),
        }
    }
}

impl SessionTree {
    /// Create a new session tree with the given root message.
    pub fn new(root_message: ChatMessage) -> Self {
        let mut arena = Arena::new();
        let root_id = arena.new_node(TreeNodeData::new(root_message.clone()));
        let mut id_index = HashMap::new();
        id_index.insert(root_message.id.clone(), root_id);
        Self {
            arena,
            root_id: Some(root_id),
            current_branch: Vec::new(),
            id_index,
            cached_filter: Mutex::new(None),
        }
    }

    /// Create a session tree from a flat list of messages.
    /// Each message becomes a child of the previous, forming a linear tree.
    pub fn from_messages(messages: &[ChatMessage]) -> Self {
        if messages.is_empty() {
            return Self::default();
        }

        let mut arena = Arena::new();
        let mut id_index = HashMap::new();
        let mut current_branch = Vec::new();

        // Create root node
        let root_id = arena.new_node(TreeNodeData::new(messages[0].clone()));
        id_index.insert(messages[0].id.clone(), root_id);

        let mut parent = root_id;

        for msg in &messages[1..] {
            let node_id = arena.new_node(TreeNodeData::new(msg.clone()));
            id_index.insert(msg.id.clone(), node_id);
            parent.append(node_id, &mut arena);
            current_branch.push(node_id);
            parent = node_id;
        }

        Self {
            arena,
            root_id: Some(root_id),
            current_branch,
            id_index,
            cached_filter: Mutex::new(None),
        }
    }

    /// Serialize the tree to a snapshot (stable across serialization).
    pub fn to_snapshot(&self) -> SessionTreeSnapshot {
        // Collect all nodes by walking the tree
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut root_id = String::new();

        // Use the existing walk() method to iterate all nodes
        for (_, node_id) in self.walk() {
            let data = self.arena.get(node_id).expect("node should exist").get().clone();
            let msg_id = data.message.id.clone();

            if self.root_id == Some(node_id) {
                root_id = msg_id.clone();
            }

            // Collect edges (parent -> child)
            for child_id in node_id.children(&self.arena) {
                if let Some(child_node) = self.arena.get(child_id) {
                    edges.push((msg_id.clone(), child_node.get().message.id.clone()));
                }
            }

            nodes.push(SerializedNode {
                id: msg_id,
                data,
            });
        }

        // Convert current_branch from NodeId to message IDs
        let current_branch: Vec<String> = self
            .current_branch
            .iter()
            .filter_map(|id| self.arena.get(*id).map(|n| n.get().message.id.clone()))
            .collect();

        SessionTreeSnapshot {
            current_branch,
            root_id,
            nodes,
            edges,
        }
    }

    /// Deserialize from a snapshot, rebuilding the arena.
    pub fn from_snapshot(snapshot: &SessionTreeSnapshot) -> Option<Self> {
        if snapshot.nodes.is_empty() {
            return Some(Self::default());
        }

        let mut arena = Arena::new();
        let mut id_to_node: HashMap<String, NodeId> = HashMap::new();

        // First pass: create all nodes
        for node in &snapshot.nodes {
            let node_id = arena.new_node(node.data.clone());
            id_to_node.insert(node.id.clone(), node_id);
        }

        // Second pass: attach children to parents
        for (parent_id, child_id) in &snapshot.edges {
            if let (Some(&parent_node), Some(&child_node)) =
                (id_to_node.get(parent_id), id_to_node.get(child_id))
            {
                parent_node.append(child_node, &mut arena);
            }
        }

        // Build current_branch from message IDs
        let current_branch: Vec<NodeId> = snapshot
            .current_branch
            .iter()
            .filter_map(|id| id_to_node.get(id).copied())
            .collect();

        // Root ID
        let root_id = id_to_node.get(&snapshot.root_id).copied();

        Some(Self {
            arena,
            root_id,
            current_branch,
            id_index: id_to_node,
            cached_filter: Mutex::new(None),
        })
    }

    /// Get the number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.arena.iter().count()
    }

    /// Get the root node ID.
    pub fn root_id(&self) -> Option<NodeId> {
        self.root_id
    }

    /// Get the first child of a node.
    #[cfg(test)]
    pub fn first_child(&self, node_id: NodeId) -> Option<NodeId> {
        node_id.children(&self.arena).next()
    }

    /// Get the node at the end of the current branch.
    pub fn current_node(&self) -> Option<&TreeNodeData> {
        self.current_branch
            .last()
            .and_then(|id| self.arena.get(*id).map(|n| n.get()))
    }

    /// Get a mutable reference to the node at the end of the current branch.
    pub fn current_node_mut(&mut self) -> Option<&mut TreeNodeData> {
        self.current_branch
            .last_mut()
            .and_then(|id| self.arena.get_mut(*id).map(|n| n.get_mut()))
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&TreeNodeData> {
        self.arena.get(id).map(|n| n.get())
    }

    /// Fork a new branch from the message at `message_index`.
    /// The new branch starts as a child of that message node.
    /// Returns the path to the new branch.
    pub fn fork_at(&mut self, message_index: usize) -> Option<Vec<NodeId>> {
        let target_id = self.node_at_message_index(message_index)?;

        // Create placeholder child to mark the fork point
        let placeholder = ChatMessage {
            role: Role::System,
            timestamp: crate::message::now(),
            id: format!("fork.{}", message_index),
            parts: vec![crate::message::Part::Text {
                content: "[fork point]".to_owned(),
            }],
            ..Default::default()
        };

        let fork_id = self.arena.new_node(TreeNodeData::new(placeholder.clone()));
        self.id_index.insert(placeholder.id.clone(), fork_id);
        if let Some(target) = target_id {
            target.append(fork_id, &mut self.arena);
        }

        // Build path to new fork
        let mut path = self.path_to_message_index(message_index)?;
        path.push(fork_id);

        self.invalidate_cache();
        Some(path)
    }

    /// Navigate to a path in the tree using NodeId lookup.
    pub fn navigate_to(&mut self, path: &[NodeId]) {
        // Validate all nodes exist in the tree
        if path.iter().all(|id| self.arena.get(*id).is_some()) {
            self.current_branch = path.to_vec();
        }
    }

    /// Get the NodeId at a flat message index (0-based) along current branch.
    fn node_at_message_index(&self, index: usize) -> Option<Option<NodeId>> {
        let path = self.path_to_message_index(index)?;
        path.last().copied().map(Some)
    }

    /// Build the path (vec of NodeId) to the message at a given flat index.
    fn path_to_message_index(&self, index: usize) -> Option<Vec<NodeId>> {
        let root_id = self.root_id?;

        if index == 0 {
            return Some(vec![root_id]);
        }

        let mut path = vec![root_id];
        let mut current = root_id;

        for _ in 1..=index {
            let children: Vec<NodeId> = current.children(&self.arena).collect();
            if children.is_empty() {
                return None;
            }
            // Follow current branch if available, otherwise first child
            let branch_idx = path.len() - 1;
            let idx = if branch_idx < self.current_branch.len() {
                // Find which child of current leads toward current_branch[branch_idx]
                let target = self.current_branch[branch_idx];
                children.iter().position(|&c| c == target).unwrap_or(0)
            } else {
                0
            };
            let child = children[idx];
            path.push(child);
            current = child;
        }

        Some(path)
    }

    /// Collect all nodes in the tree in pre-order, returning (depth, node) pairs.
    pub fn walk(&self) -> Vec<(usize, NodeId)> {
        let mut result = Vec::new();
        if let Some(root_id) = self.root_id {
            self.walk_node(root_id, 0, &mut result);
        }
        result
    }

    fn walk_node(&self, node_id: NodeId, depth: usize, result: &mut Vec<(usize, NodeId)>) {
        result.push((depth, node_id));
        for child in node_id.children(&self.arena) {
            self.walk_node(child, depth + 1, result);
        }
    }

    /// Find the path (NodeId) to the node whose message has the given id.
    pub fn find_path_by_id(&self, id: &str) -> Option<Vec<NodeId>> {
        let node_id = self.id_index.get(id).copied()?;
        Some(self.path_from_root(node_id))
    }

    /// Build path from root to a node.
    fn path_from_root(&self, target: NodeId) -> Vec<NodeId> {
        let mut path = Vec::new();
        let mut current = Some(target);
        while let Some(id) = current {
            path.push(id);
            current = self.arena.get(id).and_then(|n| n.parent());
        }
        path.reverse();
        path
    }

    /// Collect visible nodes given a filter, with caching.
    pub fn filtered_walk(&self, filter: SessionTreeFilter) -> Vec<(usize, NodeId)> {
        // Try cache first
        if let Ok(cache) = self.cached_filter.try_lock() {
            if let Some((cached_filter, cached_nodes)) = cache.as_ref() {
                if *cached_filter == filter {
                    return cached_nodes.clone();
                }
            }
        }

        // Compute fresh result
        let all = self.walk();
        let result: Vec<_> = all
            .into_iter()
            .filter(|(_, id)| {
                let node = self.arena.get(*id);
                node.map(|n| filter.passes(&n.get().message, n.get().label.as_deref()))
                    .unwrap_or(false)
            })
            .collect();

        // Store in cache
        if let Ok(mut cache) = self.cached_filter.try_lock() {
            *cache = Some((filter, result.clone()));
        }

        result
    }

    /// Invalidate the filter cache after tree mutation.
    fn invalidate_cache(&mut self) {
        if let Ok(mut cache) = self.cached_filter.try_lock() {
            *cache = None;
        }
    }

    /// Get the current branch as a vector of node IDs.
    pub fn current_branch(&self) -> &[NodeId] {
        &self.current_branch
    }

    /// Get the id_index for external lookup.
    pub fn id_index(&self) -> &HashMap<String, NodeId> {
        &self.id_index
    }

    /// Get the arena for iteration.
    pub fn arena(&self) -> &Arena<TreeNodeData> {
        &self.arena
    }

    /// Check if the tree contains a node.
    pub fn contains_node(&self, id: NodeId) -> bool {
        self.arena.get(id).is_some()
    }

    /// Get current branch length (for tests).
    #[cfg(test)]
    pub fn current_branch_len(&self) -> usize {
        self.current_branch.len()
    }

    /// Check if current branch is empty (for tests).
    #[cfg(test)]
    pub fn is_current_branch_empty(&self) -> bool {
        self.current_branch.is_empty()
    }

    /// Set label on a node (for tests).
    #[cfg(test)]
    pub fn set_node_label(&mut self, node_id: NodeId, label: Option<String>) {
        if let Some(node) = self.arena.get_mut(node_id) {
            node.get_mut().label = label;
        }
    }
}
