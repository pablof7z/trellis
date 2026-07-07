use std::collections::BTreeSet;

use trellis_core::{
    HostResourceOutcome, ResourceCommand, Revision, TransactionId, TransactionResult,
};

use crate::HostStatusEvent;

/// Previewed or committed host-facing resource plan for one app step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostPlanRecord<C = ()> {
    /// Stable app-owned step name.
    pub step: String,
    /// Ordered resource commands produced by graph propagation.
    pub commands: Vec<ResourceCommand<C>>,
}

/// Host effect application observed at an app-owned executor seam.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostEffectRecord<C = ()> {
    /// Stable app-owned step name.
    pub step: String,
    /// Declared host executor that applied the effect.
    pub executor: String,
    /// Transaction whose committed plan authorized this effect.
    pub transaction_id: TransactionId,
    /// Graph revision whose committed plan authorized this effect.
    pub revision: Revision,
    /// Resource command applied by the host executor.
    pub command: ResourceCommand<C>,
}

/// Host-conformance assertion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostConformanceError<C = ()> {
    /// A committed plan had no preview record for the same step.
    MissingPreview {
        /// Step missing preview evidence.
        step: String,
    },
    /// A committed plan did not match the previewed plan for the same step.
    CommitDiffersFromPreview {
        /// Step whose plans differed.
        step: String,
        /// Previewed commands.
        previewed: Vec<ResourceCommand<C>>,
        /// Committed commands.
        committed: Vec<ResourceCommand<C>>,
    },
    /// An applied host effect had no matching committed command.
    EffectWithoutCommit {
        /// Effect that lacked commit evidence.
        effect: HostEffectRecord<C>,
    },
    /// An applied host effect used an executor outside the declared boundary.
    EffectFromUndeclaredExecutor {
        /// Step that applied the effect.
        step: String,
        /// Executor that was not declared.
        executor: String,
    },
    /// A scanned effect site was not declared as an executor.
    UndeclaredEffectSite {
        /// Effect site found by the app's scan hook.
        site: String,
    },
    /// A host status event did not match any recorded host effect.
    StatusWithoutEffect {
        /// Status that lacked effect evidence.
        status: HostStatusEvent,
    },
}

/// Data-only ledger for asserting preview-to-commit-to-host-effect evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostConformanceLedger<C = ()> {
    previews: Vec<HostPlanRecord<C>>,
    commits: Vec<HostPlanRecord<C>>,
    effects: Vec<HostEffectRecord<C>>,
    statuses: Vec<HostStatusEvent>,
    declared_executors: BTreeSet<String>,
    observed_effect_sites: BTreeSet<String>,
}

impl<C> Default for HostConformanceLedger<C> {
    fn default() -> Self {
        Self {
            previews: Vec::new(),
            commits: Vec::new(),
            effects: Vec::new(),
            statuses: Vec::new(),
            declared_executors: BTreeSet::new(),
            observed_effect_sites: BTreeSet::new(),
        }
    }
}

impl<C> HostConformanceLedger<C> {
    /// Creates an empty host-conformance ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares an app-owned executor allowed to apply host effects.
    pub fn declare_executor(&mut self, executor: impl Into<String>) {
        self.declared_executors.insert(executor.into());
    }

    /// Records an effect site found by an app-owned static or runtime scan hook.
    pub fn record_effect_site(&mut self, site: impl Into<String>) {
        self.observed_effect_sites.insert(site.into());
    }

    /// Records one applied host effect.
    pub fn record_effect(
        &mut self,
        step: impl Into<String>,
        executor: impl Into<String>,
        transaction_id: TransactionId,
        revision: Revision,
        command: ResourceCommand<C>,
    ) {
        let executor = executor.into();
        self.observed_effect_sites.insert(executor.clone());
        self.effects.push(HostEffectRecord {
            step: step.into(),
            executor,
            transaction_id,
            revision,
            command,
        });
    }

    /// Records a host acknowledgement or failure event.
    pub fn record_status(&mut self, status: HostStatusEvent) {
        self.statuses.push(status);
    }

    /// Records the resource plan predicted by `Transaction::preview`.
    pub fn record_preview(&mut self, step: impl Into<String>, result: &TransactionResult<C>)
    where
        C: Clone,
    {
        self.previews.push(plan_record(step, result));
    }

    /// Records the resource plan returned by a committed transaction.
    pub fn record_commit(&mut self, step: impl Into<String>, result: &TransactionResult<C>)
    where
        C: Clone,
    {
        self.commits.push(plan_record(step, result));
    }

    /// Records every command in a committed result as an applied host effect.
    pub fn record_effects_from_commit(
        &mut self,
        step: impl Into<String>,
        executor: impl Into<String>,
        result: &TransactionResult<C>,
    ) where
        C: Clone,
    {
        let step = step.into();
        let executor = executor.into();
        for command in result.resource_plan.commands() {
            self.record_effect(
                step.clone(),
                executor.clone(),
                result.transaction_id,
                result.revision,
                command.clone(),
            );
        }
    }
}

impl<C: Clone + Eq> HostConformanceLedger<C> {
    /// Asserts every committed plan has matching preview evidence.
    pub fn assert_commits_match_previews(&self) -> Result<(), HostConformanceError<C>> {
        for commit in &self.commits {
            let Some(preview) = self
                .previews
                .iter()
                .find(|record| record.step == commit.step)
            else {
                return Err(HostConformanceError::MissingPreview {
                    step: commit.step.clone(),
                });
            };
            if preview.commands != commit.commands {
                return Err(HostConformanceError::CommitDiffersFromPreview {
                    step: commit.step.clone(),
                    previewed: preview.commands.clone(),
                    committed: commit.commands.clone(),
                });
            }
        }
        Ok(())
    }

    /// Asserts every applied effect was present in the committed plan.
    pub fn assert_effects_match_commits(&self) -> Result<(), HostConformanceError<C>> {
        for (index, effect) in self.effects.iter().enumerate() {
            let Some(commit) = self
                .commits
                .iter()
                .find(|record| record.step == effect.step)
            else {
                return Err(HostConformanceError::EffectWithoutCommit {
                    effect: effect.clone(),
                });
            };
            let allowed = commit
                .commands
                .iter()
                .filter(|command| *command == &effect.command)
                .count();
            let used = self.effects[..=index]
                .iter()
                .filter(|other| other.step == effect.step && other.command == effect.command)
                .count();
            if used > allowed {
                return Err(HostConformanceError::EffectWithoutCommit {
                    effect: effect.clone(),
                });
            }
        }
        Ok(())
    }

    /// Asserts observed effect sites and applied effects use declared executors.
    pub fn assert_effects_use_declared_executors(&self) -> Result<(), HostConformanceError<C>> {
        for site in &self.observed_effect_sites {
            if !self.declared_executors.contains(site) {
                return Err(HostConformanceError::UndeclaredEffectSite { site: site.clone() });
            }
        }
        for effect in &self.effects {
            if !self.declared_executors.contains(&effect.executor) {
                return Err(HostConformanceError::EffectFromUndeclaredExecutor {
                    step: effect.step.clone(),
                    executor: effect.executor.clone(),
                });
            }
        }
        Ok(())
    }

    /// Asserts host status events correspond to recorded effects.
    pub fn assert_statuses_follow_effects(&self) -> Result<(), HostConformanceError<C>> {
        for status in &self.statuses {
            if !self
                .effects
                .iter()
                .any(|effect| effect_matches_status(effect, status))
            {
                return Err(HostConformanceError::StatusWithoutEffect {
                    status: status.clone(),
                });
            }
        }
        Ok(())
    }

    /// Asserts the preview, commit, executor, and effect evidence chain.
    pub fn assert_host_seam_conforms(&self) -> Result<(), HostConformanceError<C>> {
        self.assert_commits_match_previews()?;
        self.assert_effects_match_commits()?;
        self.assert_effects_use_declared_executors()
    }
}

fn plan_record<C: Clone>(
    step: impl Into<String>,
    result: &TransactionResult<C>,
) -> HostPlanRecord<C> {
    HostPlanRecord {
        step: step.into(),
        commands: result.resource_plan.commands().to_vec(),
    }
}

fn effect_matches_status<C>(effect: &HostEffectRecord<C>, status: &HostStatusEvent) -> bool {
    effect.revision == status.command_revision
        && effect.command.key() == &status.resource_key
        && effect.command.scope() == status.scope
        && outcome_matches_command(&effect.command, &status.status)
}

fn outcome_matches_command<C>(command: &ResourceCommand<C>, outcome: &HostResourceOutcome) -> bool {
    match outcome {
        HostResourceOutcome::Open => !matches!(command, ResourceCommand::Close { .. }),
        HostResourceOutcome::Closed => matches!(command, ResourceCommand::Close { .. }),
        HostResourceOutcome::Unknown
        | HostResourceOutcome::Unsupported(_)
        | HostResourceOutcome::Failed(_) => true,
    }
}
