use core::fmt::Write;

use trellis_core::HostResourceOutcome;

use crate::{ResourceLedger, TraceRedactor};

impl<C> ResourceLedger<C> {
    /// Returns deterministic redacted debug output for resource ledger snapshots.
    pub fn to_redacted_debug_string(&self, redactor: &impl TraceRedactor) -> String {
        let mut out = String::new();
        writeln!(&mut out, "ResourceLedger").expect("writing to String cannot fail");

        writeln!(&mut out, "Live:").expect("writing to String cannot fail");
        for (key, snapshot) in &self.resources {
            writeln!(
                &mut out,
                "  key={:?} owners={:?} open={} close={} replace={} revision={:?} generation={}",
                redactor.resource_key(key),
                snapshot.owners,
                snapshot.open_count,
                snapshot.close_count,
                snapshot.replace_count,
                snapshot.command_revision,
                snapshot.generation
            )
            .expect("writing to String cannot fail");
        }

        writeln!(&mut out, "History:").expect("writing to String cannot fail");
        for (key, snapshot) in &self.history {
            writeln!(
                &mut out,
                "  key={:?} owners={:?} revision={:?} generation={}",
                redactor.resource_key(key),
                snapshot.owners,
                snapshot.command_revision,
                snapshot.generation
            )
            .expect("writing to String cannot fail");
        }

        writeln!(&mut out, "Commands:").expect("writing to String cannot fail");
        for command in &self.command_trace {
            writeln!(
                &mut out,
                "  key={:?} scope={:?} kind={:?}",
                redactor.resource_key(&command.key),
                command.scope,
                command.kind
            )
            .expect("writing to String cannot fail");
        }

        writeln!(&mut out, "Status:").expect("writing to String cannot fail");
        for record in &self.status_records {
            writeln!(
                &mut out,
                "  class={:?} key={:?} scope={:?} command_revision={:?} status_revision={:?} outcome={}",
                record.class,
                redactor.resource_key(&record.status.resource_key),
                record.status.scope,
                record.status.command_revision,
                record.status.status_revision,
                status_kind(&record.status.status)
            )
            .expect("writing to String cannot fail");
        }

        out
    }
}

fn status_kind(status: &HostResourceOutcome) -> &'static str {
    match status {
        HostResourceOutcome::Unknown => "Unknown",
        HostResourceOutcome::Open => "Open",
        HostResourceOutcome::Failed(_) => "Failed",
        HostResourceOutcome::Closed => "Closed",
        HostResourceOutcome::Unsupported(_) => "Unsupported",
    }
}
