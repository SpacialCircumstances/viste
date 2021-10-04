use log::info;
use std::collections::vec_deque::VecDeque;
use std::fmt::{Debug, Display, Formatter};
use std::mem::replace;
use std::ops::{Index, IndexMut};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NodeIndex(usize);

impl Display for NodeIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
struct Adjacency {
    parents: Vec<usize>,
    children: Vec<usize>,
}

enum Node<T> {
    Empty,
    Filled(T, Adjacency),
}

pub struct Graph<T> {
    nodes: Vec<Node<T>>,
    free_nodes: VecDeque<usize>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SearchContinuation<T> {
    Continue(T),
    Stop,
}

fn remove_from<T: Eq>(vec: &mut Vec<T>, element: T) {
    if let Some(child_pos) = vec.iter().position(|x| *x == element) {
        vec.swap_remove(child_pos);
    }
}

impl<T> Graph<T> {
    pub fn new() -> Self {
        Graph {
            nodes: vec![],
            free_nodes: VecDeque::new(),
        }
    }

    fn get_adjacency_mut(&mut self, node: usize) -> &mut Adjacency {
        match &mut self.nodes[node] {
            Node::Empty => panic!("Expected filled node"),
            Node::Filled(_, adj) => adj,
        }
    }

    fn get_adjacency(&self, node: usize) -> &Adjacency {
        match &self.nodes[node] {
            Node::Empty => panic!("Expected filled node"),
            Node::Filled(_, adj) => adj,
        }
    }

    fn try_get_adjacency(&self, node: usize) -> Option<&Adjacency> {
        match &self.nodes[node] {
            Node::Empty => None,
            Node::Filled(_, adj) => Some(adj),
        }
    }

    fn get_data(&self, node: usize) -> &T {
        match &self.nodes[node] {
            Node::Empty => panic!("Expected filled node"),
            Node::Filled(data, _) => data,
        }
    }

    fn get_data_mut(&mut self, node: usize) -> &mut T {
        match &mut self.nodes[node] {
            Node::Empty => panic!("Expected filled node"),
            Node::Filled(data, _) => data,
        }
    }

    pub fn has_edge(&self, from: NodeIndex, to: NodeIndex) -> bool {
        if let Some(adj) = self.try_get_adjacency(from.0) {
            adj.children.contains(&to.0)
        } else {
            false
        }
    }

    pub fn has_node(&self, node: NodeIndex) -> bool {
        self.try_get_adjacency(node.0).is_some()
    }

    pub fn add_node(&mut self, value: T) -> NodeIndex {
        let node = Node::Filled(
            value,
            Adjacency {
                parents: Vec::new(),
                children: Vec::new(),
            },
        );
        let ni = match self.free_nodes.pop_front() {
            None => {
                self.nodes.push(node);
                self.nodes.len() - 1
            }
            Some(free_idx) => match self.nodes[free_idx] {
                Node::Filled(_, _) => panic!("Expected empty node"),
                Node::Empty => {
                    self.nodes[free_idx] = node;
                    free_idx
                }
            },
        };
        info!("Added node {}", ni);
        NodeIndex(ni)
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex) {
        info!("Adding edge {} -> {}", from, to);
        self.get_adjacency_mut(from.0).children.push(to.0);
        self.get_adjacency_mut(to.0).parents.push(from.0);
    }

    pub fn remove_edge(&mut self, from: NodeIndex, to: NodeIndex) {
        info!("Removing edge {} -> {}", from, to);
        let from_adj = self.get_adjacency_mut(from.0);
        remove_from(&mut from_adj.children, to.0);

        let to_adj = self.get_adjacency_mut(to.0);
        remove_from(&mut to_adj.parents, from.0);
    }

    pub fn remove_node(&mut self, node: NodeIndex) -> T {
        info!("Removing node {}", node);
        match replace(&mut self.nodes[node.0], Node::Empty) {
            Node::Filled(data, adj) => {
                for ch in &adj.children {
                    remove_from(&mut self.get_adjacency_mut(*ch).parents, node.0);
                }
                for p in &adj.parents {
                    remove_from(&mut self.get_adjacency_mut(*p).children, node.0);
                }
                self.free_nodes.push_back(node.0);
                data
            }
            Node::Empty => panic!("Expected filled node"),
        }
    }

    pub fn search_children<C: Copy, F: FnMut(&T, NodeIndex, C) -> SearchContinuation<C>>(
        &self,
        mut searcher: F,
        start_node: NodeIndex,
        initial_state: C,
    ) {
        let mut to_search = VecDeque::new();
        to_search.push_back((start_node.0, initial_state));

        while let Some((idx, state)) = to_search.pop_front() {
            if let SearchContinuation::Continue(new_state) =
                searcher(self.get_data(idx), NodeIndex(idx), state)
            {
                for child in &self.get_adjacency(idx).children {
                    to_search.push_back((*child, new_state));
                }
            }
        }
    }

    pub fn search_children_mut<C: Copy, F: FnMut(&mut T, NodeIndex, C) -> SearchContinuation<C>>(
        &mut self,
        mut searcher: F,
        start_node: NodeIndex,
        initial_state: C,
    ) {
        let mut to_search = VecDeque::new();
        to_search.push_back((start_node.0, initial_state));

        while let Some((idx, state)) = to_search.pop_front() {
            if let SearchContinuation::Continue(new_state) =
                searcher(self.get_data_mut(idx), NodeIndex(idx), state)
            {
                for child in &self.get_adjacency(idx).children {
                    to_search.push_back((*child, new_state));
                }
            }
        }
    }
}

impl<T: Debug> Debug for Graph<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (idx, node) in self.nodes.iter().enumerate() {
            if let Node::Filled(t, adj) = node {
                writeln!(f, "{}: {:?} ({:?})", idx, t, adj)?;
            }
        }
        Ok(())
    }
}

impl<T> Default for Graph<T> {
    fn default() -> Self {
        Graph::new()
    }
}

impl<T> Index<NodeIndex> for Graph<T> {
    type Output = T;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        self.get_data(index.0)
    }
}

impl<T> IndexMut<NodeIndex> for Graph<T> {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
        self.get_data_mut(index.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::Graph;

    #[test]
    fn test_adding() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(2);
        let n2 = graph.add_node(3);
        graph.add_edge(n1, n2);
        assert!(graph.has_edge(n1, n2));
        assert_eq!(graph.has_edge(n2, n1), false);
    }

    #[test]
    fn test_removal() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(2);
        let n2 = graph.add_node(3);
        graph.add_edge(n1, n2);
        graph.remove_edge(n1, n2);
        assert_eq!(graph.has_edge(n1, n2), false);
        assert!(graph.has_node(n1));
        assert!(graph.has_node(n2));
        graph.remove_node(n1);
        graph.remove_node(n2);
        assert_eq!(graph.has_node(n1), false);
        assert_eq!(graph.has_node(n2), false);
    }

    #[test]
    fn test_add_remove_and_edge() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(3);
        let n2 = graph.add_node(4);
        let n3 = graph.add_node(5);
        graph.add_edge(n1, n2);
        graph.add_edge(n1, n3);
        graph.add_edge(n2, n3);
        graph.remove_node(n3);
        let n4 = graph.add_node(5);
        assert_eq!(graph.has_node(n4), true);
        assert_eq!(graph.has_edge(n1, n3), false);
        assert_eq!(graph.has_edge(n1, n2), true);
        assert_eq!(graph.has_edge(n1, n4), false);
    }
}
