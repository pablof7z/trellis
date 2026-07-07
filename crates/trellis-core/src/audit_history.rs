use crate::{
    AuditExplanationLevel, AuditExplanations, NodeChangeExplanation, NodeHandle, NodeId,
    OutputFrameExplanation, OutputKey, ResourceCommandExplanation, ResourceKey, Revision,
    TransactionResult,
};
use std::collections::BTreeMap;

/// Host-retained audit explanation receipts keyed by graph revision.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuditHistory {
    revisions: BTreeMap<Revision, Vec<AuditExplanations>>,
}

impl AuditHistory {
    /// Creates an empty retained audit history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retains the audit explanations from a transaction result.
    pub fn retain<C>(&mut self, result: &TransactionResult<C>) {
        self.retain_explanations(result.audit_explanations.clone());
    }

    /// Retains an audit explanation receipt without requiring full transaction payloads.
    pub fn retain_explanations(&mut self, explanations: AuditExplanations) {
        self.revisions
            .entry(explanations.revision)
            .or_default()
            .push(explanations);
    }

    /// Removes every retained receipt for a revision.
    pub fn remove_revision(&mut self, revision: Revision) -> Vec<AuditExplanations> {
        self.revisions.remove(&revision).unwrap_or_default()
    }

    /// Clears all retained audit receipts.
    pub fn clear(&mut self) {
        self.revisions.clear();
    }

    /// Explains why a typed node changed at a retained revision.
    pub fn why_at<H: NodeHandle>(
        &self,
        revision: Revision,
        node: H,
    ) -> Result<&NodeChangeExplanation, AuditHistoryError> {
        self.why_changed_by_id_at(revision, node.id())
    }

    /// Explains why a node id changed at a retained revision.
    pub fn why_changed_by_id_at(
        &self,
        revision: Revision,
        node: NodeId,
    ) -> Result<&NodeChangeExplanation, AuditHistoryError> {
        let receipts = self.receipts_for(revision)?;
        let mut saw_enabled = false;
        for receipt in receipts {
            if receipt.level == AuditExplanationLevel::Disabled {
                continue;
            }
            saw_enabled = true;
            if let Some(explanation) = receipt.node_changes.get(&node) {
                return Ok(explanation);
            }
        }
        if saw_enabled {
            Err(AuditHistoryError::NodeChangeNotFound { revision, node })
        } else {
            Err(AuditHistoryError::ExplanationsDisabled { revision })
        }
    }

    /// Explains why a resource command was emitted at a retained revision.
    pub fn why_resource_command_at(
        &self,
        revision: Revision,
        key: &ResourceKey,
    ) -> Result<&ResourceCommandExplanation, AuditHistoryError> {
        let receipts = self.receipts_for(revision)?;
        let mut saw_enabled = false;
        for receipt in receipts {
            if receipt.level == AuditExplanationLevel::Disabled {
                continue;
            }
            saw_enabled = true;
            if let Some(explanation) = receipt.resource_commands.get(key) {
                return Ok(explanation);
            }
        }
        if saw_enabled {
            Err(AuditHistoryError::ResourceCommandNotFound {
                revision,
                key: key.clone(),
            })
        } else {
            Err(AuditHistoryError::ExplanationsDisabled { revision })
        }
    }

    /// Explains why an output frame was emitted at a retained revision.
    pub fn why_output_frame_at(
        &self,
        revision: Revision,
        output_key: OutputKey,
    ) -> Result<&OutputFrameExplanation, AuditHistoryError> {
        let receipts = self.receipts_for(revision)?;
        let mut saw_enabled = false;
        for receipt in receipts {
            if receipt.level == AuditExplanationLevel::Disabled {
                continue;
            }
            saw_enabled = true;
            if let Some(explanation) = receipt.output_frames.get(&output_key) {
                return Ok(explanation);
            }
        }
        if saw_enabled {
            Err(AuditHistoryError::OutputFrameNotFound {
                revision,
                output_key,
            })
        } else {
            Err(AuditHistoryError::ExplanationsDisabled { revision })
        }
    }

    /// Returns a retained dependency path at a revision.
    pub fn dependency_path_at(
        &self,
        revision: Revision,
        from: NodeId,
        to: NodeId,
    ) -> Result<Vec<NodeId>, AuditHistoryError> {
        let receipts = self.receipts_for(revision)?;
        let mut saw_enabled = false;
        let mut saw_path_enabled = false;
        for receipt in receipts {
            match receipt.level {
                AuditExplanationLevel::Disabled => continue,
                AuditExplanationLevel::Summary => {
                    saw_enabled = true;
                }
                AuditExplanationLevel::DependencyPaths => {
                    saw_enabled = true;
                    saw_path_enabled = true;
                    if let Some(path) = retained_dependency_path(receipt, from, to) {
                        return Ok(path);
                    }
                }
            }
        }
        if !saw_enabled {
            Err(AuditHistoryError::ExplanationsDisabled { revision })
        } else if !saw_path_enabled {
            Err(AuditHistoryError::DependencyPathsNotRetained { revision })
        } else {
            Err(AuditHistoryError::DependencyPathNotFound { revision, from, to })
        }
    }

    fn receipts_for(&self, revision: Revision) -> Result<&[AuditExplanations], AuditHistoryError> {
        self.revisions
            .get(&revision)
            .map(Vec::as_slice)
            .ok_or(AuditHistoryError::RevisionNotRetained { revision })
    }
}

/// Reason a historical audit query could not be answered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditHistoryError {
    /// No retained receipt exists for the requested revision.
    RevisionNotRetained {
        /// Requested graph revision.
        revision: Revision,
    },
    /// The retained receipt explicitly disabled audit explanations.
    ExplanationsDisabled {
        /// Requested graph revision.
        revision: Revision,
    },
    /// The retained receipt kept summaries but not dependency paths.
    DependencyPathsNotRetained {
        /// Requested graph revision.
        revision: Revision,
    },
    /// No retained node-change explanation matched the query.
    NodeChangeNotFound {
        /// Requested graph revision.
        revision: Revision,
        /// Requested node.
        node: NodeId,
    },
    /// No retained resource-command explanation matched the query.
    ResourceCommandNotFound {
        /// Requested graph revision.
        revision: Revision,
        /// Requested resource key.
        key: ResourceKey,
    },
    /// No retained output-frame explanation matched the query.
    OutputFrameNotFound {
        /// Requested graph revision.
        revision: Revision,
        /// Requested output key.
        output_key: OutputKey,
    },
    /// No retained dependency path matched the query.
    DependencyPathNotFound {
        /// Requested graph revision.
        revision: Revision,
        /// Requested upstream node.
        from: NodeId,
        /// Requested downstream node.
        to: NodeId,
    },
}

fn retained_dependency_path(
    receipt: &AuditExplanations,
    from: NodeId,
    to: NodeId,
) -> Option<Vec<NodeId>> {
    let node_paths = receipt
        .node_changes
        .values()
        .flat_map(|explanation| explanation.dependency_paths.iter());
    let resource_paths = receipt
        .resource_commands
        .values()
        .flat_map(|explanation| explanation.dependency_paths.iter());
    let output_paths = receipt
        .output_frames
        .values()
        .flat_map(|explanation| explanation.dependency_paths.iter());
    node_paths
        .chain(resource_paths)
        .chain(output_paths)
        .find(|path| path.first() == Some(&from) && path.last() == Some(&to))
        .cloned()
}
