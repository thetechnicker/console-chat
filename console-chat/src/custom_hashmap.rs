//! This is a custom implementation of a Prefix tree.
//! its whole purpose is for me to learn how to do low level memory in rust
//!
//! This is not a general purpose trie! it is meant to be static after creation.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::pin::Pin;

const MAX_KEY_LEN: usize = 256;

struct Value<'a, T> {
    ptr: *const T,
    _phantom: PhantomData<&'a T>,
}

/// Do not use this in your code.
impl<'a, T> Value<'a, T> {
    #[allow(dead_code)]
    pub fn new(value: &'a T) -> Self {
        Self {
            ptr: value as *const T,
            _phantom: PhantomData,
        }
    }

    pub fn from_raw(value: *const T) -> Self {
        Self {
            ptr: value,
            _phantom: PhantomData,
        }
    }

    unsafe fn get_unchecked(&self) -> &'a T {
        unsafe { &*self.ptr }
    }

    pub fn get(&self) -> Option<&'a T> {
        unsafe {
            if !self.ptr.is_null() {
                return Some(self.get_unchecked());
            }
        }
        None
    }
}

impl<'a, T> std::fmt::Debug for Value<'a, T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Value")
            .field("content", &self.get())
            .field("raw", &self.ptr)
            .finish()
    }
}

type CharMap<'a> = HashMap<char, Node<'a>>;

#[derive(Debug)]
enum Node<'a> {
    Value(Value<'a, String>),
    Map(CharMap<'a>), // for simplicity, fixed branching
}
impl<'a> Node<'a> {
    pub fn map(&self) -> Option<&CharMap<'a>> {
        match self {
            Node::Map(map) => Some(map),
            Node::Value(_) => None,
        }
    }

    pub fn value(&self) -> Option<Result<&str, String>> {
        match self {
            Node::Map(_) => None,
            Node::Value(value) => {
                if let Some(value) = value.get() {
                    Some(Ok(value))
                } else {
                    Some(Err("the value is a invalid pointer".to_string()))
                }
            }
        }
    }
}

fn build_prefix_path_recursive<'a>(
    key: &mut VecDeque<char>,
    map: &mut CharMap<'a>,
    end: Value<'a, String>,
) -> Result<(), String> {
    if key.len() > MAX_KEY_LEN {
        return Err(format!(
            "Key is longer than the maximum: key {}, max {}",
            key.len(),
            MAX_KEY_LEN
        ));
    }
    if let Some(c) = key.pop_front() {
        if let Some(existing_path) = map.get_mut(&c) {
            match existing_path {
                Node::Map(map) => build_prefix_path_recursive(key, map, end)?,
                Node::Value(_) => return Err("Path Already Exists".to_string()),
            }
        } else {
            let mut end = Node::Value(end);
            while let Some(c) = key.pop_back() {
                end = Node::Map(HashMap::from([(c, end)]));
            }
            map.insert(c, end);
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct Trie<'a> {
    values: Pin<Box<[String]>>,
    root: CharMap<'a>,
}

impl<'a> Trie<'a> {
    pub fn new(values: Vec<(&[char], String)>) -> Result<Self, String> {
        let mut value_box = Pin::new(vec![String::new(); values.len()].into_boxed_slice());
        let mut root = CharMap::new();
        for (i, (key, value)) in values.iter().enumerate() {
            let mut key = VecDeque::from(key.to_vec());
            value_box[i] = value.to_owned();
            let mapped_value = Value::from_raw(&value_box[i]);
            build_prefix_path_recursive(&mut key, &mut root, mapped_value)?;
        }
        Ok(Self {
            values: value_box,
            root,
        })
    }

    pub fn get_values(&self) -> &[String] {
        &self.values
    }

    fn get_node(&self, c: char) -> Option<&Node<'a>> {
        self.root.get(&c)
    }
    pub fn traverse(self) -> TrieTraverser<'a> {
        TrieTraverser {
            root: self,
            node: None,
        }
    }
}

#[allow(dead_code)]
enum TraversalResult2<'a> {
    Map(TrieTraverser<'a>),
    Value(String, Trie<'a>),
    Error(Trie<'a>),
}

pub enum TraversalResult {
    MappingNode,
    UnusedPath,
    Value(String),
}

pub struct TrieTraverser<'a> {
    root: Trie<'a>,
    node: Option<Value<'a, Node<'a>>>,
}

impl<'a> TrieTraverser<'a> {
    #[allow(dead_code)]
    fn next_consuming(mut self, c: char) -> TraversalResult2<'a> {
        if let Some(node) = self.node.take() {
            if let Some(map) = node.get().unwrap().map() {
                match map.get(&c) {
                    Some(node) => self.node = Some(Value::from_raw(node)),
                    None => return TraversalResult2::Error(self.root),
                }
            } else {
                panic!("This is invalid");
            }
        } else {
            self.node = self.root.get_node(c).map(|node| Value::from_raw(node));
        }
        if let Some(Node::Value(v)) = self.node.as_ref().map(|n| n.get().unwrap()) {
            let str = v.get().map(|str| str.clone()).unwrap();
            return TraversalResult2::Value(str, self.root);
        }
        TraversalResult2::Map(self)
    }

    pub fn next(&mut self, c: char) -> Result<TraversalResult, String> {
        if let Some(node_ptr) = self.node.take() {
            match node_ptr.get().unwrap() {
                Node::Map(map) => match map.get(&c) {
                    Some(node) => self.node = Some(Value::from_raw(node)),
                    None => {
                        self.node = None;
                        return Ok(TraversalResult::UnusedPath);
                    }
                },
                Node::Value(_) => {
                    return Err("Invalid State, the current node is a value".to_string());
                }
            }
        } else {
            self.node = self.root.get_node(c).map(|node| Value::from_raw(node));
        }
        if let Some(Node::Value(v)) = self.node.as_ref().map(|node| node.get().unwrap()) {
            let str = v.get().map(|str| str.clone()).unwrap();
            self.node = None;
            return Ok(TraversalResult::Value(str));
        }
        Ok(TraversalResult::MappingNode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_tree() {
        let trie = Trie::new(vec![
            (&['a', 'a', 'a'], "Test".to_owned()),
            (&['a', 'a', 'b'], "Test".to_owned()),
            (&['a', 'a', 'c'], "Test".to_owned()),
            (&['a', 'b', 'a'], "Test".to_owned()),
            (&['a', 'b', 'b'], "Test".to_owned()),
            (&['a', 'b', 'c'], "Test".to_owned()),
            (&['a', 'c', 'a'], "Test".to_owned()),
            (&['a', 'c', 'b'], "Test".to_owned()),
            (&['a', 'c', 'c'], "Test".to_owned()),
        ])
        .unwrap();

        let node = trie.get_node('a');
        assert!(matches!(node, Some(Node::Map(_))));
        let map = node.unwrap().map().unwrap();

        let node = map.get(&'a');
        assert!(matches!(node, Some(Node::Map(_))));
        let map = node.unwrap().map().unwrap();

        let node = map.get(&'a');
        assert!(matches!(node, Some(Node::Value(_))));
        let map = node.unwrap().value().unwrap();
        assert_eq!(map, Ok(trie.values[0].as_str()));
    }

    #[test]
    fn traversal_test() {
        let trie = Trie::new(vec![
            (&['a', 'a', 'a'], "Test".to_owned()),
            (&['a', 'a', 'b'], "Test".to_owned()),
            (&['a', 'a', 'c'], "Test".to_owned()),
            (&['a', 'b', 'a'], "Test".to_owned()),
            (&['a', 'b', 'b'], "Test".to_owned()),
            (&['a', 'b', 'c'], "Test".to_owned()),
            (&['a', 'c', 'a'], "Test".to_owned()),
            (&['a', 'c', 'b'], "Test".to_owned()),
            (&['a', 'c', 'c'], "Test".to_owned()),
            (&['b', 'a', 'a'], "Test".to_owned()),
            (&['b', 'a', 'b'], "Test".to_owned()),
            (&['b', 'a', 'c'], "Test".to_owned()),
            (&['b', 'b', 'a'], "Test".to_owned()),
            (&['b', 'b', 'b'], "Test".to_owned()),
            (&['b', 'b', 'c'], "Test".to_owned()),
            (&['b', 'c', 'a'], "Test".to_owned()),
            (&['b', 'c', 'b'], "Test".to_owned()),
            (&['b', 'c', 'c'], "Test".to_owned()),
            (&['c', 'a', 'a'], "Test".to_owned()),
            (&['c', 'a', 'b'], "Test".to_owned()),
            (&['c', 'a', 'c'], "Test".to_owned()),
            (&['c', 'b', 'a'], "Test".to_owned()),
            (&['c', 'b', 'b'], "Test".to_owned()),
            (&['c', 'b', 'c'], "Test".to_owned()),
            (&['c', 'c', 'a'], "Test".to_owned()),
            (&['c', 'c', 'b'], "Test".to_owned()),
            (&['c', 'c', 'c'], "Test".to_owned()),
        ])
        .unwrap();

        let mut traveler = trie.traverse();
        for c in "abc".chars() {
            match traveler.next(c) {
                Ok(_) => {}
                Err(_) => panic!(),
            }
        }
    }
}
