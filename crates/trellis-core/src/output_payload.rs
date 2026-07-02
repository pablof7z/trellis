use core::any::Any;
use core::fmt;

pub(crate) trait StoredOutput: Any + Send + Sync {
    fn clone_box(&self) -> Box<dyn StoredOutput>;
    fn equals(&self, other: &dyn StoredOutput) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn type_name(&self) -> &'static str;
}

impl Clone for Box<dyn StoredOutput> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Clone)]
pub(crate) struct OutputValue<T> {
    value: T,
}

impl<T> OutputValue<T> {
    pub(crate) fn new(value: T) -> Self {
        Self { value }
    }

    pub(crate) fn get(&self) -> &T {
        &self.value
    }
}

impl<T> StoredOutput for OutputValue<T>
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<dyn StoredOutput> {
        Box::new(self.clone())
    }

    fn equals(&self, other: &dyn StoredOutput) -> bool {
        other
            .as_any()
            .downcast_ref::<OutputValue<T>>()
            .is_some_and(|other| self.value == other.value)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        core::any::type_name::<T>()
    }
}

/// Type-erased materialized output payload carried by an output frame.
#[derive(Clone)]
pub struct OutputPayload {
    value: Box<dyn StoredOutput>,
}

impl OutputPayload {
    /// Creates an erased output payload from a typed value.
    pub fn new<T>(value: T) -> Self
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        Self {
            value: Box::new(OutputValue::new(value)),
        }
    }

    pub(crate) fn from_stored(value: Box<dyn StoredOutput>) -> Self {
        Self { value }
    }

    /// Returns this payload as the requested type, if it matches.
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        self.value
            .as_any()
            .downcast_ref::<OutputValue<T>>()
            .map(OutputValue::get)
    }

    /// Returns the erased Rust type name carried by this payload.
    pub fn type_name(&self) -> &'static str {
        self.value.type_name()
    }
}

impl fmt::Debug for OutputPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputPayload")
            .field("type_name", &self.type_name())
            .finish_non_exhaustive()
    }
}

impl PartialEq for OutputPayload {
    fn eq(&self, other: &Self) -> bool {
        self.value.equals(other.value.as_ref())
    }
}

pub(crate) fn boxed_output<T>(value: T) -> Box<dyn StoredOutput>
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    Box::new(OutputValue::new(value))
}
