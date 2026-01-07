use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct TypeErasedWrapper {
    data: Arc<Box<dyn std::any::Any + Send + Sync>>,
    debug_full: String,
    debug_normal: String,
}

impl TypeErasedWrapper {
    pub fn new<T: 'static + Clone + Send + Sync + std::fmt::Debug>(value: T) -> Self {
        let debug_full = format!("{:#?}", value);
        let debug_normal = format!("{:?}", value);
        TypeErasedWrapper {
            data: Arc::new(Box::new(value)),
            debug_full,
            debug_normal,
        }
    }

    #[allow(dead_code)]
    pub fn downcast<T: 'static + Clone>(&self) -> Result<T, &Self> {
        if let Some(value) = self.data.downcast_ref::<T>() {
            tracing::debug!("YAY");
            Ok(value.clone())
        } else {
            tracing::debug!("NAY");
            Err(self)
        }
    }
}

impl std::fmt::Debug for TypeErasedWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Any {}", self.debug_full)
    }
}

impl std::fmt::Display for TypeErasedWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Any {}", self.debug_normal)
    }
}

impl Deref for TypeErasedWrapper {
    type Target = Box<dyn std::any::Any + Send + Sync>; // Added + Send and + Sync
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use color_eyre::Result;

    #[test]
    fn test_type_erased_wrapper() -> Result<()> {
        let x = 1;
        let y = TypeErasedWrapper::new(x);
        let z = y.downcast::<u8>();
        assert!(z.is_ok(), "{:?}", z);
        assert_eq!(z.expect("TypeErasedWrapper Failed"), x);
        Ok(())
    }
}
