//! This is a custom implem//! This is a custom implementation of a Prefix tree.
//! its whole purpose is for me to learn how to do low level memory in rust
//!
//! Plan:
//! key: [`&[char]`], value: [`String`]
//! getting value example:
//! 'a' -> 'b' -> 'c' -> "value"
//! 'a' -> 'c' -> 'c' -> "other value"

use std::collections::HashMap;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::pin::Pin;

const MAX_KEY_LEN: usize = 256;

pub struct Value<'a, T> {
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
pub enum Node<'a> {
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

#[derive(Debug)]
pub struct Root<'a> {
    values: Pin<Box<[String]>>,
    root: CharMap<'a>,
}

// TODO: return result to explain error, maybe 3 state enum ok, length, existing sub path ends
// earlier than new path
fn build_prefix_path_recursive<'a>(
    key: &mut VecDeque<char>,
    map: &mut CharMap<'a>,
    end: Value<'a, String>,
) {
    if key.len() > MAX_KEY_LEN {
        panic!()
    }
    if let Some(c) = key.pop_front() {
        if let Some(existing_path) = map.get_mut(&c) {
            match existing_path {
                Node::Map(map) => build_prefix_path_recursive(key, map, end),
                Node::Value(_) => panic!(),
            }
        } else {
            let mut end = Node::Value(end);
            while let Some(c) = key.pop_back() {
                end = Node::Map(HashMap::from([(c, end)]));
            }
            map.insert(c, end);
        }
    }
}

impl<'a> Root<'a> {
    pub fn new(values: Vec<(&[char], String)>) -> Self {
        let mut value_box = Pin::new(vec![String::new(); values.len()].into_boxed_slice());
        let mut root = CharMap::new();
        for (i, (key, value)) in values.iter().enumerate() {
            let mut key = VecDeque::from(key.to_vec());
            value_box[i] = value.to_owned();
            let mapped_value = Value::from_raw(&value_box[i]);
            build_prefix_path_recursive(&mut key, &mut root, mapped_value);
        }
        Self {
            values: value_box,
            root,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_tree() {
        let trie = Root::new(vec![
            (&['a', 'a', 'a'], "Test".to_owned()),
            (&['a', 'a', 'b'], "Test".to_owned()),
            (&['a', 'a', 'c'], "Test".to_owned()),
            (&['a', 'b', 'a'], "Test".to_owned()),
            (&['a', 'b', 'b'], "Test".to_owned()),
            (&['a', 'b', 'c'], "Test".to_owned()),
            (&['a', 'c', 'a'], "Test".to_owned()),
            (&['a', 'c', 'b'], "Test".to_owned()),
            (&['a', 'c', 'c'], "Test".to_owned()),
        ]);

        let node = trie.root.get(&'a');
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
}
