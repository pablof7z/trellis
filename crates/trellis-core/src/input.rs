use core::any::{Any, TypeId};

pub(crate) trait StoredInput: Any {
    fn clone_box(&self) -> Box<dyn StoredInput>;
    fn equals(&self, other: &dyn StoredInput) -> bool;
    fn as_any(&self) -> &dyn Any;
}

impl Clone for Box<dyn StoredInput> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Clone)]
pub(crate) struct InputValue<T> {
    value: T,
}

impl<T> InputValue<T> {
    pub(crate) fn new(value: T) -> Self {
        Self { value }
    }

    pub(crate) fn get(&self) -> &T {
        &self.value
    }
}

impl<T> StoredInput for InputValue<T>
where
    T: Clone + PartialEq + 'static,
{
    fn clone_box(&self) -> Box<dyn StoredInput> {
        Box::new(self.clone())
    }

    fn equals(&self, other: &dyn StoredInput) -> bool {
        other
            .as_any()
            .downcast_ref::<InputValue<T>>()
            .is_some_and(|other| self.value == other.value)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub(crate) fn boxed_input<T>(value: T) -> Box<dyn StoredInput>
where
    T: Clone + PartialEq + 'static,
{
    Box::new(InputValue::new(value))
}

pub(crate) fn downcast_input<T>(value: &dyn StoredInput) -> Option<&T>
where
    T: Clone + PartialEq + 'static,
{
    value
        .as_any()
        .downcast_ref::<InputValue<T>>()
        .map(InputValue::get)
}

pub(crate) fn value_type<T>() -> TypeId
where
    T: 'static,
{
    TypeId::of::<T>()
}
