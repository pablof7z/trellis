use core::fmt::{Debug, Write};

use crate::OutputLedger;

impl<O> OutputLedger<O> {
    /// Returns deterministic debug output for output ledger snapshots.
    pub fn to_debug_string(&self) -> String
    where
        O: Debug,
    {
        self.to_redacted_debug_string(|value| format!("{value:?}"))
    }

    /// Returns deterministic redacted debug output for output ledger snapshots.
    pub fn to_redacted_debug_string(&self, redact: impl Fn(&O) -> String) -> String {
        let mut out = String::new();
        writeln!(&mut out, "OutputLedger").expect("writing to String cannot fail");

        writeln!(&mut out, "Outputs:").expect("writing to String cannot fail");
        for (key, snapshot) in &self.outputs {
            let state = snapshot
                .state
                .as_ref()
                .map(&redact)
                .unwrap_or_else(|| "None".to_owned());
            writeln!(
                &mut out,
                "  key={key:?} scope={:?} tx={:?} revision={:?} cleared={} state={}",
                snapshot.scope, snapshot.transaction_id, snapshot.revision, snapshot.cleared, state
            )
            .expect("writing to String cannot fail");
        }

        writeln!(&mut out, "Closed scopes: {:?}", self.closed_scopes)
            .expect("writing to String cannot fail");
        writeln!(&mut out, "Errors: {:?}", self.errors).expect("writing to String cannot fail");
        out
    }
}
