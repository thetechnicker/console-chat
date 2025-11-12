use std::cell::RefCell;
use std::rc::Rc;
use std::slice::Iter;

use crate::widgets::Widget;

#[derive(Debug, Clone)]
pub enum WidgetElement {
    Item(Rc<RefCell<dyn Widget>>),
    Collection(Rc<[WidgetElement]>),
    /// This WidgetElement contains a element that has a length itself
    CollectionWithLongElement((Rc<[WidgetElement]>, usize)),
}

use std::ops::Index;

impl Index<usize> for WidgetElement {
    type Output = WidgetElement;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            WidgetElement::Item(_) => panic!("Can't index an item"),
            WidgetElement::Collection(collection) => &collection[index],
            WidgetElement::CollectionWithLongElement(collection) => &collection.0[index],
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
            WidgetElement::CollectionWithLongElement(collection) => {
                stack.push(collection.0.iter());
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
                    WidgetElement::CollectionWithLongElement(collection) => {
                        // Push the iterator of this collection onto stack
                        self.stack.push(collection.0.iter());
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
            WidgetElement::Item(i) => i.borrow().get_len(),
            WidgetElement::Collection(c) => c.len(),
            WidgetElement::CollectionWithLongElement((c, long_item)) => {
                c.len() + c[*long_item].num_rows()
            }
        }
    }
    pub fn num_col(&self, row: usize) -> usize {
        match self {
            WidgetElement::Item(i) => i.borrow().get_len(),
            WidgetElement::Collection(c) => c[row].num_rows(),
            WidgetElement::CollectionWithLongElement((c, long_item)) => {
                c.len() + c[*long_item].num_rows()
            }
        }
    }

    pub fn get_widget(&self, indecies: &[usize]) -> Option<Rc<RefCell<dyn Widget>>> {
        let mut current_item: Self = self.clone();
        for index in indecies {
            current_item = match current_item {
                Self::Item(item) => Self::Item(item.clone()),
                Self::Collection(collection) => collection[*index].clone(),
                Self::CollectionWithLongElement(collection) => collection.0[*index].clone(),
            };
            if let Self::Item(item) = current_item {
                return Some(item);
            }
        }
        None
    }

    pub fn get_item_2d(&self, row: usize, column: usize) -> Option<Rc<RefCell<dyn Widget>>> {
        match self {
            WidgetElement::CollectionWithLongElement((c, _)) | WidgetElement::Collection(c) => {
                match c[row].clone() {
                    WidgetElement::CollectionWithLongElement((c, _))
                    | WidgetElement::Collection(c) => match c[column].clone() {
                        WidgetElement::CollectionWithLongElement(_)
                        | WidgetElement::Collection(_) => None,
                        WidgetElement::Item(item) => Some(item.clone()),
                    },
                    WidgetElement::Item(item) => Some(item.clone()),
                }
            }
            WidgetElement::Item(item) => Some(item.clone()),
        }
    }
}
