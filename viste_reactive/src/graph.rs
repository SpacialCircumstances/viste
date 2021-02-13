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
}
