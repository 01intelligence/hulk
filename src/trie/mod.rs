use std::collections::HashMap;

// A trie container.
#[derive(Default)]
pub struct Trie {
    root: Node,
    size: usize,
}

// Trie tree node container carries value and children.
#[derive(Default)]
pub struct Node {
    exists: bool,
    value: String,
    child: HashMap<char, Node>,
}

impl Trie {
    // Returns root node.
    pub fn root(&self) -> &Node {
        &self.root
    }

    // Insert a key.
    pub fn insert(&mut self, key: String) {
        let mut cur_node = &mut self.root;
        for v in key.chars() {
            if !cur_node.child.contains_key(&v) {
                cur_node.child.insert(v, Node::default());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Ensure that we can insert new keys into the tree, then check the size.
    fn test_trie_insert() {
        let mut trie = Trie::default();

        trie.insert("key".to_string());
        trie.insert("keyy".to_string());

        // After inserting, we should have a size of two.
        assert_eq!(trie.size, 2, "expected size 2, got: {}", trie.size)
    }

    #[test]
    // Ensure that prefix_match gives us the correct two keys in the tree.
    fn test_trie_prefix_match() {
        let mut trie = Trie::default();

        // Feed it some fodder: only "hulk" and "hulky-o's" should trip the matcher.
        trie.insert("hulk".to_string());
        trie.insert("amazon".to_string());
        trie.insert("cheerio".to_string());
        trie.insert("hulky-o's".to_string());

        let matches = trie.prefix_match("hul");
        assert_eq!(
            matches.len(),
            2,
            "expected two matches, got: {}",
            matches.len()
        );

        if matches[0] != "hulk" && matches[1] != "hulk" {
            panic!(
                "expected one match to be 'hulk', got: '{}' and '{}'",
                matches[0], matches[1]
            )
        }
    }
}
