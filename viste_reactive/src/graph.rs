use std::mem::replace;
use std::ops::Index;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct NodeIndex(usize);

struct Adjacency {
    parents: Vec<usize>,
    children: Vec<usize>,
}

enum Node<T> {
    Empty(Option<usize>),
    Filled(T, Adjacency),
}

pub struct Graph<T> {
    first_empty_index: usize,
    nodes: Vec<Node<T>>,
}

impl<T> Graph<T> {
    pub fn new() -> Self {
        Graph {
            first_empty_index: 0,
            nodes: vec![],
        }
    }

    fn get_adjacency_mut(&mut self, node: usize) -> &mut Adjacency {
        match &mut self.nodes[node] {
            Node::Empty(_) => panic!("Expected filled node"),
            Node::Filled(_, adj) => adj,
        }
    }

    fn get_adjacency(&self, node: usize) -> &Adjacency {
        match &self.nodes[node] {
            Node::Empty(_) => panic!("Expected filled node"),
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
        let res_idx = NodeIndex(self.first_empty_index);
        if self.first_empty_index >= self.nodes.len() {
            self.nodes.push(node);
            self.first_empty_index += 1;
        } else {
            match self.nodes[self.first_empty_index] {
                Node::Filled(_, _) => panic!("Expected empty node"),
                Node::Empty(next) => {
                    self.nodes[self.first_empty_index] = node;
                    match next {
                        None => self.first_empty_index = self.nodes.len(),
                        Some(next) => self.first_empty_index = next,
                    }
                }
            }
        }
        res_idx
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
        //TODO: Find next empty node
        match replace(&mut self.nodes[node.0], Node::Empty(None)) {
            Node::Filled(data, adj) => {
                for ch in &adj.children {
                    self.get_adjacency_mut(*ch).parents.remove(node.0);
                }
                for p in &adj.parents {
                    self.get_adjacency_mut(*p).children.remove(node.0);
                }
                data
            }
            Node::Empty(_) => panic!("Expected filled node"),
        }
    }
}

impl<T> Index<NodeIndex> for Graph<T> {
    type Output = T;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        match &self.nodes[index.0] {
            Node::Filled(val, _) => val,
            Node::Empty(_) => panic!("Expected filled node"),
        }
    }
}
