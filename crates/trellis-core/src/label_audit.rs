use crate::{
    AuditExplanationsTrace, GraphLabelRegistry, NodeId,
    labels::{node_fallback, output_fallback, scope_fallback},
};

impl GraphLabelRegistry {
    pub(crate) fn include_audit_explanation_defaults(
        &mut self,
        explanations: &AuditExplanationsTrace,
    ) {
        for explanation in &explanations.node_changes {
            self.label_node_if_absent(explanation.node, node_fallback(explanation.node));
            self.include_node_list_defaults(&explanation.input_causes);
            self.include_path_defaults(&explanation.dependency_paths);
        }
        for explanation in &explanations.resource_commands {
            self.label_resource_if_absent(
                explanation.key.clone(),
                explanation.key.as_str().to_owned(),
            );
            self.label_scope_if_absent(explanation.scope, scope_fallback(explanation.scope));
            self.include_node_list_defaults(&explanation.collection_diffs);
            self.include_node_list_defaults(&explanation.changed_nodes);
            self.include_node_list_defaults(&explanation.input_causes);
            self.include_path_defaults(&explanation.dependency_paths);
        }
        for explanation in &explanations.output_frames {
            self.label_output_if_absent(
                explanation.output_key,
                output_fallback(explanation.output_key),
            );
            self.label_scope_if_absent(explanation.scope, scope_fallback(explanation.scope));
            self.include_node_list_defaults(&explanation.dependencies);
            self.include_node_list_defaults(&explanation.changed_dependencies);
            self.include_node_list_defaults(&explanation.input_causes);
            self.include_path_defaults(&explanation.dependency_paths);
        }
    }

    fn include_node_list_defaults(&mut self, nodes: &[NodeId]) {
        for node in nodes {
            self.label_node_if_absent(*node, node_fallback(*node));
        }
    }

    fn include_path_defaults(&mut self, paths: &[Vec<NodeId>]) {
        for path in paths {
            self.include_node_list_defaults(path);
        }
    }
}
