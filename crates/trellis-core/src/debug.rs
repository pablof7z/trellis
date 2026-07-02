use crate::Graph;
use core::fmt::Write;

impl<C> Graph<C> {
    /// Returns a deterministic text dump of graph metadata.
    pub fn debug_dump(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "Graph(revision={})", self.revision().get())
            .expect("writing to String cannot fail");

        writeln!(&mut out, "Scopes:").expect("writing to String cannot fail");
        for scope in self.scopes() {
            writeln!(
                &mut out,
                "  {:?} name={:?} parent={:?} closed={}",
                scope.id(),
                scope.debug_name(),
                scope.parent(),
                scope.is_closed()
            )
            .expect("writing to String cannot fail");
        }

        writeln!(&mut out, "Nodes:").expect("writing to String cannot fail");
        for node in self.nodes() {
            writeln!(
                &mut out,
                "  {:?} kind={:?} name={:?} scope={:?} deps={:?}",
                node.id(),
                node.kind(),
                node.debug_name(),
                node.owning_scope(),
                node.dependencies().as_slice()
            )
            .expect("writing to String cannot fail");
        }

        writeln!(&mut out, "Dependency paths:").expect("writing to String cannot fail");
        for node in self.nodes() {
            for dependency in node.dependencies().as_slice() {
                writeln!(&mut out, "  {dependency:?} -> {:?}", node.id())
                    .expect("writing to String cannot fail");
            }
        }

        writeln!(&mut out, "Resources:").expect("writing to String cannot fail");
        for (key, owners) in &self.resource_owners {
            writeln!(&mut out, "  {key:?} owners={owners:?}")
                .expect("writing to String cannot fail");
        }

        writeln!(&mut out, "Outputs:").expect("writing to String cannot fail");
        for output in self.outputs.values() {
            writeln!(
                &mut out,
                "  {:?} name={:?} scope={:?} deps={:?}",
                output.key(),
                output.debug_name(),
                output.scope(),
                output.dependencies().as_slice()
            )
            .expect("writing to String cannot fail");
        }

        writeln!(&mut out, "Audit:").expect("writing to String cannot fail");
        for entry in &self.audit.log {
            writeln!(
                &mut out,
                "  tx={:?} revision={:?} event={:?}",
                entry.transaction_id, entry.revision, entry.event
            )
            .expect("writing to String cannot fail");
        }

        out
    }
}
