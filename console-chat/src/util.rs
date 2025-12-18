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
    data: Arc<Box<dyn std::any::Any + Send + Sync>>,
}

impl TypeErasedWrapper {
    pub fn new<T: 'static + Send + Sync>(value: T) -> Self {
        TypeErasedWrapper {
            data: Arc::new(Box::new(value)),
        }
    }

    pub fn downcast<T: 'static>(self) -> Result<T, TypeErasedWrapper> {
        match Arc::try_unwrap(self.data) {
            Ok(value) => Ok(*value.downcast().unwrap()),
            Err(data) => Err(TypeErasedWrapper { data }),
        }
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
