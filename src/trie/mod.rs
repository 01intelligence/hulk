use std::collections::HashMap;

// A trie container.
pub struct Trie {
    root: Node,
    size: usize,
}

// Trie tree node container carries value and children.
pub struct Node {
    exists: bool,
    value: String,
    child: HashMap<char, Node>,
}

impl Trie {
    pub fn new() -> Trie {
        Trie {
            root: Node::new(),
            size: 0,
        }
    }

    // Returns root node.
    pub fn root(&self) -> &Node {
        &self.root
    }

    // Insert a key.
    pub fn insert(&mut self, key: String) {
        let mut cur_node = &mut self.root;
        for v in key.chars() {
            if cur_node.child.contains_key(&v) {
                cur_node.child.insert(v, Node::new());
            }
            cur_node = cur_node.child.get_mut(&v).unwrap();
        }

        if !cur_node.exists {
            // Increment when new rune child is added.
            self.size += 1;
            cur_node.exists = true;
        }
        // Value is stored for retrieval in future.
        cur_node.value = key;
    }

    // Prefix match.
    pub fn prefix_match(&self, key: &str) -> Vec<&str> {
        if let Some((node, _)) = self.find_node(key) {
            self.walk(node)
        } else {
            Default::default()
        }
    }

    // Walk the tree.
    pub fn walk<'a>(&'a self, node: &'a Node) -> Vec<&'a str> {
        let mut r: Vec<&str> = Vec::new();
        if node.exists {
            r.push(&node.value);
        }
        for (_, v) in node.child.iter() {
            r.extend(self.walk(v))
        }
        r
    }

    // Find node corresponding to key.
    fn find_node(&self, key: &str) -> Option<(&Node, usize)> {
        let mut cur_node = &self.root;
        let mut index = 0;
        let mut f = false;
        for (k, v) in key.chars().enumerate() {
            if f {
                index = k;
                f = false;
            }
            if let Some(c) = cur_node.child.get(&v) {
                cur_node = c;
            } else {
                return None;
            }
            if cur_node.exists {
                f = true;
            }
        }

        if cur_node.exists {
            index = key.chars().count();
        }
        Some((cur_node, index))
    }
}

impl Node {
    fn new() -> Node {
        Node {
            exists: false,
            value: "".to_string(),
            child: Default::default(),
        }
    }
}