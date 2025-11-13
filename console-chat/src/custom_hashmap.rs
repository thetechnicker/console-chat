//! This is a custom implem//! This is a custom implementation of a Prefix tree.
//! its whole purpose is for me to learn how to do low level memory in rust

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::Index;
use std::ops::IndexMut;
use std::rc::Rc;

const ZERO: usize = '0' as usize;
const MAX: usize = 'z' as usize;

const ALPHABET_LEN: usize = MAX - ZERO + 1;
fn char_to_index(c: char) -> usize {
    let c = c as usize;
    if MAX < c || c < ZERO {
        panic!(
            "Char outside of range: min: {}, max: {}, c: {}",
            MAX, ZERO, c
        );
    }
    c - ZERO
}

pub type Values = Rc<[String]>;
pub type MapInner<'a> = [TrieNode<'a>; ALPHABET_LEN];

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct Map<'a> {
    inner: MapInner<'a>,
}

impl<'a> Default for Map<'a> {
    fn default() -> Self {
        Self {
            inner: std::array::from_fn(|_| TrieNode::default()),
        }
    }
}

impl<'a> Index<char> for Map<'a> {
    type Output = TrieNode<'a>;
    fn index(&self, c: char) -> &TrieNode<'a> {
        &self.inner[char_to_index(c)]
    }
}
impl<'a> IndexMut<char> for Map<'a> {
    fn index_mut(&mut self, c: char) -> &mut TrieNode<'a> {
        &mut self.inner[char_to_index(c)]
    }
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct Value<'a, T> {
    ptr: *const T,
    marker: PhantomData<&'a T>,
}

impl<'a, T> Value<'a, T> {
    /// Create a new Value from a reference with lifetime `'a`
    pub fn new(reference: &T) -> Self {
        Self {
            ptr: reference as *const T,
            marker: PhantomData,
        }
    }
    pub fn get_value(&self) -> Option<&T> {
        unsafe {
            if !self.ptr.is_null() {
                return Some(&*self.ptr);
            }
        }
        None
    }
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub enum TrieNode<'a> {
    #[default]
    None,
    Value(Value<'a, String>),
    Map(Rc<Map<'a>>),
}

#[derive(Debug, Clone)]
pub struct Trie<'a> {
    root: Rc<Map<'a>>,
    values: Values,
}

impl<'a> Trie<'a> {
    pub fn new(mapped_values: &[(Vec<char>, String)]) -> Self {
        let mut root = Map::default();
        let mut values = Vec::with_capacity(mapped_values.len());
        for (_, value) in mapped_values.iter() {
            values.push(value.to_owned());
        }
        let boxed_values = values.into_boxed_slice();
        let values: Values = Rc::from(boxed_values);
        //let mut root_2: Map<'a> = std::array::from_fn(|_| TrieNode::default());
        for (i, (key, _)) in mapped_values.iter().enumerate() {
            let mut key = VecDeque::from(key.to_owned());
            let idk = Value::new(&values[i]);
            let mut end = TrieNode::Value(idk);
            let start: char = key.pop_front().unwrap_or('0');
            for c in key.iter().rev() {
                let mut map = Map::default();
                map[*c] = end;
                end = TrieNode::Map(Rc::new(map));
            }
            root[start] = end;
        }
        let this = Self {
            root: Rc::new(root),
            values,
        };
        this
    }
    pub fn get_node(&self, c: char) -> &TrieNode<'a> {
        &self.root[c]
    }
    pub fn get_values(&self) -> Rc<[String]> {
        self.values.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn test_trie() {
        let key = vec!['a', 'b', 'c'];
        let trie = Trie::new(&[(key, "ABC".into())]);
        let mut node = trie.get_node('a');
        assert!(matches!(node, TrieNode::Map(_)));
        if let TrieNode::Map(map) = node {
            node = &map['b'];
        }
        assert!(matches!(node, TrieNode::Map(_)));
        if let TrieNode::Map(map) = node {
            node = &map['c'];
        }
        assert!(matches!(node, TrieNode::Value(_)));

        if let TrieNode::Value(v) = node {
            assert_eq!(v.get_value(), Some("ABC".to_string()).as_ref());
        }
    }

    #[test]
    fn test_value_new_and_deref() {
        // Create an Rc slice of Strings
        let values: Rc<[String]> = Rc::from(vec!["a".into(), "b".into(), "c".into()]);

        // Create a Value pointing to the second element
        let val = Value::new(&values[1]);

        // Safety: val.ptr points to values[1], which is still alive
        unsafe {
            assert_eq!(&*val.ptr, "b");
        }
    }
}
