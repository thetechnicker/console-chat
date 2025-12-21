use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub struct TypeErasedWrapper {
    data: Arc<Box<dyn std::any::Any + Send + Sync>>,
    debug_full: String,
    debug_normal: String,
}

unsafe impl Send for TypeErasedWrapper {}
unsafe impl Sync for TypeErasedWrapper {}

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

//use serde::Deserialize;
//use serde::ser::*;
//#[derive(Deserialize, Debug, PartialEq, Eq)]
//pub struct OptionalBool(Option<bool>);
//
//impl Deref for OptionalBool {
//    type Target = bool;
//    fn deref(&self) -> &Self::Target {
//        self.0.as_ref().unwrap_or(&false)
//    }
//}
//impl DerefMut for OptionalBool {
//    fn deref_mut(&mut self) -> &mut Self::Target {
//        self.0.get_or_insert(false)
//    }
//}
//
//impl Serialize for OptionalBool {
//    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//    where
//        S: Serializer,
//    {
//        match self.0 {
//            Some(value) => serializer.serialize_bool(value),
//            None => serializer.serialize_none(),
//        }
//    }
//}

#[cfg(test)]
mod test {
    use super::*;
    use color_eyre::Result;

    //#[test]
    //fn test_optional_bool() -> Result<()> {
    //    let mut test = OptionalBool(None);
    //    assert!(!*test);
    //    *test = true;
    //    assert!(*test);
    //    let str = serde_json::to_string(&test)?;
    //    let back = serde_json::from_str::<OptionalBool>(&str)?;
    //    assert_eq!(back, test);
    //    Ok(())
    //}

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
