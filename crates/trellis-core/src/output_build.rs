use crate::{
    DependencyList, GraphError, GraphResult, MaterializedOutput, OutputContext, OutputError,
    OutputMeta, OutputOptions, RebaselineReason, Transaction, output::OutputSpec,
};

impl<C: 'static, O> Transaction<'_, C, O>
where
    O: Clone + PartialEq + 'static,
{
    /// Stages creation of a materialized output with default options.
    pub fn materialized_output(
        &mut self,
        debug_name: impl Into<String>,
        scope: crate::ScopeId,
        dependencies: DependencyList,
        materialize: impl for<'ctx> Fn(&OutputContext<'ctx, C, O>) -> Result<O, OutputError> + 'static,
    ) -> GraphResult<MaterializedOutput<O>> {
        self.materialized_output_with_options(
            debug_name,
            scope,
            dependencies,
            OutputOptions::default(),
            materialize,
        )
    }

    /// Stages creation of a materialized output with explicit options.
    pub fn materialized_output_with_options(
        &mut self,
        debug_name: impl Into<String>,
        scope: crate::ScopeId,
        dependencies: DependencyList,
        options: OutputOptions,
        materialize: impl for<'ctx> Fn(&OutputContext<'ctx, C, O>) -> Result<O, OutputError> + 'static,
    ) -> GraphResult<MaterializedOutput<O>> {
        self.ensure_open()?;
        self.working.require_scope_open(scope)?;
        let key = self.graph.allocate_output_key();
        self.working.validate_output_dependencies(&dependencies)?;
        self.working.outputs.insert(
            key,
            OutputMeta::new(
                key,
                debug_name,
                scope,
                dependencies.clone(),
                options,
                self.working.revision,
            ),
        );
        self.working
            .output_specs
            .insert(key, OutputSpec::new(materialize));
        self.graph_mutated = true;
        Ok(MaterializedOutput::new(key))
    }

    /// Stages an explicit output rebaseline.
    pub fn rebaseline_output(&mut self, output: MaterializedOutput<O>) -> GraphResult<()> {
        self.ensure_open()?;
        let meta = self
            .working
            .output_meta(output.key())
            .ok_or(GraphError::UnknownOutput(output.key()))?;
        self.working.require_scope_open(meta.scope())?;
        self.staged_output_rebaselines
            .insert(output.key(), RebaselineReason::Requested);
        self.graph_mutated = true;
        Ok(())
    }
}
