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
                NodeIndex(self.nodes.len())
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
        self.get_adjacency_mut(from.0).children.remove(to.0);
        self.get_adjacency_mut(to.0).parents.remove(from.0);
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
}

impl<T> Index<NodeIndex> for Graph<T> {
    type Output = T;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        match &self.nodes[index.0] {
            Node::Filled(val, _) => val,
            Node::Empty => panic!("Expected filled node"),
        }
    }
}
