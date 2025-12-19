use serde::Deserialize;
use serde::ser::*;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct OptionalBool(Option<bool>);

impl Deref for OptionalBool {
    type Target = bool;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap_or(&false)
    }
}
impl DerefMut for OptionalBool {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_or_insert(false)
    }
}

impl Serialize for OptionalBool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Some(value) => serializer.serialize_bool(value),
            None => serializer.serialize_none(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeErasedWrapper {
    data: Arc<Box<dyn std::any::Any>>,
}

unsafe impl Send for TypeErasedWrapper {}
unsafe impl Sync for TypeErasedWrapper {}

impl TypeErasedWrapper {
    pub fn new<T: 'static>(value: T) -> Self {
        TypeErasedWrapper {
            data: Arc::new(Box::new(value)),
        }
    }

    pub fn downcast<T: 'static>(self) -> Result<T, TypeErasedWrapper> {
        match Arc::try_unwrap(self.data) {
            Ok(value) => Ok(*value.downcast::<T>().map_err(|data| Self {
                data: Arc::new(data),
            })?),
            Err(data) => Err(TypeErasedWrapper { data }),
        }
    }
}

impl Deref for TypeErasedWrapper {
    type Target = Box<dyn std::any::Any>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use color_eyre::Result;
    #[test]
    fn basic() -> Result<()> {
        let mut test = OptionalBool(None);
        assert!(!*test);
        *test = true;
        assert!(*test);

        let str = serde_json::to_string(&test)?;
        let back = serde_json::from_str::<OptionalBool>(&str)?;
        assert_eq!(back, test);
        Ok(())
    }

    #[test]
    fn test_type_erased_wrapper() -> Result<()> {
        let x = 1;
        let y = TypeErasedWrapper::new(x);
        let z = y.downcast::<u8>();
        assert!(z.is_ok(), "{:?}", z);
        assert_eq!(z.unwrap(), x);
        Ok(())
    }
}

// TODO: May use tokio
struct SaveUpdateAsyncRead<T>
where
    T: Clone,
{
    data: Arc<std::sync::Mutex<T>>,
    local_copy: T,
    has_update: std::sync::atomic::AtomicBool,
}

impl<T> Clone for SaveUpdateAsyncRead<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let local_copy = self.data.lock().map_or(|data| *data, self.local_copy);
        Self {
            data: self.data.clone(),
            local_copy,
            has_update: self.has_update,
        }
    }
}
