use std::cell::RefCell;
use std::rc::Rc;
use std::slice::Iter;

use crate::widgets::Widget;

#[derive(Debug, Clone)]
pub enum WidgetElement {
    Item(Rc<RefCell<dyn Widget>>),
    Collection(Rc<[WidgetElement]>),
    /// This WidgetElement contains a element that has a length itself
    CollectionWithLongElement(Rc<[WidgetElement]>, usize),
}

impl<T> From<&[T]> for WidgetElement
where
    T: Widget + 'static + Clone,
{
    fn from(from: &[T]) -> Self {
        let from_vec: Vec<WidgetElement> = from.to_vec().into_iter().map(|i| i.into()).collect();

        let from_box = from_vec.into_boxed_slice();
        Self::Collection(Rc::from(from_box))
    }
}

impl<T, const N: usize> From<[T; N]> for WidgetElement
where
    T: Widget + 'static + Clone,
{
    fn from(from: [T; N]) -> Self {
        let mut needs_long = false;
        let mut long_index = 0;
        let from_vec: Vec<WidgetElement> = from
            .to_vec()
            .into_iter()
            .enumerate()
            .map(|(x, i)| {
                if i.is_long() {
                    needs_long = true;
                    long_index = x;
                    Self::Item(i.boxed())
                } else {
                    i.into()
                }
            })
            .collect();

        let from_box = from_vec.into_boxed_slice();
        if needs_long {
            return Self::CollectionWithLongElement(Rc::from(from_box), long_index);
        }
        Self::Collection(Rc::from(from_box))
    }
}

impl<T> From<T> for WidgetElement
where
    T: Widget + 'static,
{
    fn from(from: T) -> Self {
        if from.is_long() {
            return Self::CollectionWithLongElement(Rc::new([Self::Item(from.boxed())]), 0);
        }
        Self::Item(from.boxed())
    }
}

use std::ops::Index;

impl Index<usize> for WidgetElement {
    type Output = WidgetElement;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            WidgetElement::Item(_) => panic!("Can't index an item"),
            WidgetElement::Collection(collection) => &collection[index],
            WidgetElement::CollectionWithLongElement(collection, _) => &collection[index],
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
            WidgetElement::CollectionWithLongElement(collection, _) => {
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
                    WidgetElement::CollectionWithLongElement(collection, _) => {
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
            WidgetElement::Item(i) => i.borrow().get_len(),
            WidgetElement::Collection(c) => c.len(),
            WidgetElement::CollectionWithLongElement(c, long_item) => {
                c.len() + c[*long_item].num_rows()
            }
        }
    }
    pub fn num_col(&self, row: usize) -> usize {
        match self {
            WidgetElement::Item(i) => i.borrow().get_len(),
            WidgetElement::Collection(c) => c[row].num_rows(),
            WidgetElement::CollectionWithLongElement(c, long_item) => {
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
                Self::CollectionWithLongElement(collection, _) => collection[*index].clone(),
            };
            if let Self::Item(item) = current_item {
                return Some(item);
            }
        }
        None
    }

    pub fn get_item_2d(&self, row: usize, column: usize) -> Option<Rc<RefCell<dyn Widget>>> {
        match self {
            WidgetElement::CollectionWithLongElement(c, _) | WidgetElement::Collection(c) => {
                match c[row].clone() {
                    WidgetElement::CollectionWithLongElement(c, _)
                    | WidgetElement::Collection(c) => match c[column].clone() {
                        WidgetElement::CollectionWithLongElement(_, _)
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

#[macro_export]
macro_rules! widget_element {
    // Match an array (slice) of widgets/elements, recursively construct WidgetElement::Collection
    ([ $($elem:tt),* $(,)? ]) => {{
        let elements = vec![
            $(widget_element!($elem)),*
        ];
        WidgetElement::Collection(Rc::from(elements.into_boxed_slice()))
    }};
    // For a single widget, convert it using From implementation into WidgetElement::Item
    ($item:expr) => {{
        WidgetElement::from($item)
    }};
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test() {
        let a = crate::widgets::Button::new("abc", 'c', "abc");
        let b = crate::widgets::Button::new("abc", 'c', "abc");
        let c = crate::widgets::Button::new("abc", 'c', "abc");
        let d = crate::widgets::Button::new("abc", 'c', "abc");
        let y = widget_element!([a, [b, c], d]);
        assert!(matches!(y, WidgetElement::Collection(_)));
        assert!(matches!(y[0], WidgetElement::Item(_)));
        assert!(matches!(y[1], WidgetElement::Collection(_)));
        assert!(matches!(y[1][0], WidgetElement::Item(_)));
        assert!(matches!(y[1][1], WidgetElement::Item(_)));
        assert!(matches!(y[2], WidgetElement::Item(_)));
    }
}
