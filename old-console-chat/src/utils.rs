use std::ops::{Add, Rem, Sub};

pub fn increment_wrapping<T>(x: &mut T, max: T)
where
    T: Copy + Add<Output = T> + Rem<Output = T> + From<u8> + PartialOrd,
{
    let one = T::from(1u8);
    let zero = T::from(0u8);
    if max == zero {
        return;
    }
    *x = (*x + one) % max;
}

pub fn decrement_wrapping<T>(x: &mut T, max: T)
where
    T: Copy + Sub<Output = T> + Rem<Output = T> + From<u8> + PartialOrd,
{
    let one = T::from(1u8);
    let zero = T::from(0u8);
    if max == zero {
        return;
    }
    *x = if *x == zero { max - one } else { *x - one } % max;
}

use clippers;
use clippers::ClipperData;

pub fn get_clipboard_content() -> Option<String> {
    let mut clipboard = clippers::Clipboard::get();
    match clipboard.read() {
        Some(ClipperData::Image(_)) => None,
        Some(ClipperData::Text(text)) => Some(text.to_string()),
        Some(_) => None,
        None => None,
    }
}
