use std::cell::RefCell;
use std::rc::Rc;
use std::slice::Iter;

use crate::widgets::Widget;

#[derive(Debug, Clone)]
pub enum WidgetElement {
    Collection(Rc<[WidgetElement]>),
    Item(Rc<RefCell<dyn Widget>>),
}

use std::ops::Index;

impl Index<usize> for WidgetElement {
    type Output = WidgetElement;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            WidgetElement::Collection(collection) => &collection[index],
            WidgetElement::Item(_) => panic!("Can't index an item"),
        }
    }
}

// Iterator that recursively traverses WidgetElement and yields Rc<RefCell<dyn Widget>>
pub struct WidgetElementIter<'a> {
    // Stack of slices iterators for recursive traversal
    stack: Vec<Iter<'a, WidgetElement>>,
    // Current item if at leaf
    current_item: Option<&'a Rc<RefCell<dyn Widget>>>,
}

impl<'a> WidgetElementIter<'a> {
    pub fn new(root: &'a WidgetElement) -> Self {
        let mut stack = Vec::new();
        let current_item = match root {
            WidgetElement::Item(item) => Some(item),
            WidgetElement::Collection(collection) => {
                stack.push(collection.iter());
                None
            }
        };
        Self {
            stack,
            current_item,
        }
    }
}

impl<'a> Iterator for WidgetElementIter<'a> {
    type Item = &'a Rc<RefCell<dyn Widget>>;

    fn next(&mut self) -> Option<Self::Item> {
        // If currently at an item, return it once and clear
        if let Some(item) = self.current_item.take() {
            return Some(item);
        }

        // Otherwise, iterate through the stack of collections
        while let Some(top_iter) = self.stack.last_mut() {
            if let Some(next_element) = top_iter.next() {
                match next_element {
                    WidgetElement::Item(item) => {
                        // Return the current item reference
                        return Some(item);
                    }
                    WidgetElement::Collection(collection) => {
                        // Push the iterator of this collection onto stack
                        self.stack.push(collection.iter());
                    }
                }
            } else {
                // Current iterator exhausted, pop it off
                self.stack.pop();
            }
        }
        // Exhausted all
        None
    }
}

// Convenience method for usage
impl WidgetElement {
    pub fn iter(&self) -> WidgetElementIter<'_> {
        WidgetElementIter::new(self)
    }

    pub fn num_rows(&self) -> usize {
        match self {
            WidgetElement::Collection(c) => c.len(),
            WidgetElement::Item(_) => 0,
        }
    }
    pub fn num_col(&self, row: usize) -> usize {
        match self {
            WidgetElement::Collection(c) => c[row].num_rows(),
            WidgetElement::Item(_) => 0,
        }
    }

    pub fn get_item(&self, row: usize, column: usize) -> Option<Rc<RefCell<dyn Widget>>> {
        match self {
            WidgetElement::Collection(c) => match c[row].clone() {
                WidgetElement::Collection(c) => match c[column].clone() {
                    WidgetElement::Collection(_) => None,
                    WidgetElement::Item(item) => Some(item.clone()),
                },
                WidgetElement::Item(item) => Some(item.clone()),
            },
            WidgetElement::Item(item) => Some(item.clone()),
        }
    }
}
