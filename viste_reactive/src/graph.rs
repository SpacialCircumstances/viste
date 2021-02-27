use std::collections::vec_deque::VecDeque;
use std::mem::replace;
use std::ops::Index;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct NodeIndex(usize);

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
pub enum SearchContinuation {
    Continue,
    Stop,
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
        self.get_adjacency(from.0).children.contains(&to.0)
    }

    pub fn add_node(&mut self, value: T) -> NodeIndex {
        let node = Node::Filled(
            value,
            Adjacency {
                parents: Vec::new(),
                children: Vec::new(),
            },
        );
        match self.free_nodes.pop_front() {
            None => {
                self.nodes.push(node);
                NodeIndex(self.nodes.len() - 1)
            }
            Some(free_idx) => match self.nodes[free_idx] {
                Node::Filled(_, _) => panic!("Expected empty node"),
                Node::Empty => {
                    self.nodes[free_idx] = node;
                    NodeIndex(free_idx)
                }
            },
        }
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex) {
        self.get_adjacency_mut(from.0).children.push(to.0);
        self.get_adjacency_mut(to.0).parents.push(from.0);
    }

    pub fn remove_edge(&mut self, from: NodeIndex, to: NodeIndex) {
        let mut from_adj = self.get_adjacency_mut(from.0);
        if let Some(child_pos) = from_adj.children.iter().position(|x| *x == to.0) {
            from_adj.children.swap_remove(child_pos);
        }

        let mut to_adj = self.get_adjacency_mut(to.0);
        if let Some(parent_pos) = to_adj.parents.iter().position(|x| *x == from.0) {
            to_adj.parents.swap_remove(parent_pos);
        }
    }

    pub fn remove_node(&mut self, node: NodeIndex) -> T {
        match replace(&mut self.nodes[node.0], Node::Empty) {
            Node::Filled(data, adj) => {
                for ch in &adj.children {
                    self.get_adjacency_mut(*ch).parents.remove(node.0);
                }
                for p in &adj.parents {
                    self.get_adjacency_mut(*p).children.remove(node.0);
                }
                self.free_nodes.push_back(node.0);
                data
            }
            Node::Empty => panic!("Expected filled node"),
        }
    }

    pub fn search_children<F: FnMut(&T) -> SearchContinuation>(
        &self,
        mut searcher: F,
        start_node: NodeIndex,
    ) {
        let mut to_search = VecDeque::new();
        to_search.push_back(start_node.0);

        while let Some(n) = to_search.pop_front() {
            if searcher(self.get_data(n)) == SearchContinuation::Continue {
                for child in &self.get_adjacency(n).children {
                    to_search.push_back(*child);
                }
            }
        }
    }

    pub fn search_children_mut<F: FnMut(&mut T) -> SearchContinuation>(
        &mut self,
        mut searcher: F,
        start_node: NodeIndex,
    ) {
        let mut to_search = VecDeque::new();
        to_search.push_back(start_node.0);

        while let Some(n) = to_search.pop_front() {
            if searcher(self.get_data_mut(n)) == SearchContinuation::Continue {
                for child in &self.get_adjacency(n).children {
                    to_search.push_back(*child);
                }
            }
        }
    }
}

impl<T> Index<NodeIndex> for Graph<T> {
    type Output = T;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        self.get_data(index.0)
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
        graph.remove_node(n1);
        graph.remove_node(n2);
        assert_eq!(graph.has_edge(n1, n2), false);
    }
}
