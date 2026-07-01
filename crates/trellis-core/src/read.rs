use crate::input::{downcast_input, value_type};
use crate::{DerivedNode, Graph, GraphError, GraphResult, InputNode, NodeId, NodeKind};

impl Graph {
    /// Returns the committed value for a typed input node.
    pub fn input_value<T>(&self, input: InputNode<T>) -> GraphResult<Option<&T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.input_value_by_id(input.id())
    }

    /// Returns the committed value for an input node id.
    pub fn input_value_by_id<T>(&self, node: NodeId) -> GraphResult<Option<&T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.validate_input_write::<T>(node)?;
        Ok(self
            .input_values
            .get(&node)
            .and_then(|value| downcast_input::<T>(value.as_ref())))
    }

    /// Returns the committed value for a typed derived node.
    pub fn derived_value<T>(&self, derived: DerivedNode<T>) -> GraphResult<Option<&T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.derived_value_by_id(derived.id())
    }

    /// Returns the committed value for a derived node id.
    pub fn derived_value_by_id<T>(&self, node: NodeId) -> GraphResult<Option<&T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.validate_derived_write::<T>(node)?;
        Ok(self
            .derived_values
            .get(&node)
            .and_then(|value| downcast_input::<T>(value.as_ref())))
    }

    pub(crate) fn validate_input_write<T>(&self, node: NodeId) -> GraphResult<()>
    where
        T: 'static,
    {
        let meta = self.nodes.get(&node).ok_or(GraphError::UnknownNode(node))?;
        if meta.kind() != NodeKind::Input {
            return Err(GraphError::NotInputNode(node));
        }
        if meta.value_type() != Some(value_type::<T>()) {
            return Err(GraphError::WrongInputType(node));
        }
        Ok(())
    }

    pub(crate) fn validate_derived_write<T>(&self, node: NodeId) -> GraphResult<()>
    where
        T: 'static,
    {
        let meta = self.nodes.get(&node).ok_or(GraphError::UnknownNode(node))?;
        if meta.kind() != NodeKind::Derived {
            return Err(GraphError::NotDerivedNode(node));
        }
        if meta.value_type() != Some(value_type::<T>()) {
            return Err(GraphError::WrongDerivedType(node));
        }
        Ok(())
    }
}
