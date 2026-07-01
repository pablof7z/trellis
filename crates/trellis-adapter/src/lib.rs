//! Runtime-neutral adapter boundary helpers for Trellis.
//!
//! This crate consumes transaction-result data. It does not mutate graphs,
//! schedule work, spawn tasks, or change propagation semantics.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::convert::Infallible;

use trellis_core::{
    OutputFrame, ResourceCommand, Revision, TransactionId, TransactionResult, TransactionTrace,
};

/// Applies resource commands outside graph propagation.
pub trait ResourceCommandSink<C> {
    /// Error returned by the host sink.
    type Error;

    /// Applies one graph-produced resource command.
    fn apply(&mut self, command: ResourceCommand<C>) -> Result<(), Self::Error>;
}

/// Emits output frames outside graph propagation.
pub trait OutputFrameSink<O> {
    /// Error returned by the host sink.
    type Error;

    /// Emits one graph-produced output frame.
    fn emit(&mut self, frame: OutputFrame<O>) -> Result<(), Self::Error>;
}

/// Error returned while applying a transaction result through adapter sinks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdapterError<ResourceError, OutputError> {
    /// Resource command application failed.
    Resource(ResourceError),
    /// Output frame emission failed.
    Output(OutputError),
}

/// Summary of a transaction result consumed by an adapter boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterReceipt {
    /// Transaction that was consumed.
    pub transaction_id: TransactionId,
    /// Revision carried by the consumed result.
    pub revision: Revision,
    /// Number of resource commands applied.
    pub resource_command_count: usize,
    /// Number of output frames emitted.
    pub output_frame_count: usize,
    /// Payload-free trace of the consumed transaction result.
    pub trace: TransactionTrace,
}

/// Runtime-neutral adapter boundary over host-provided sinks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterBoundary<ResourceSink, OutputSink> {
    resource_sink: ResourceSink,
    output_sink: OutputSink,
}

impl<ResourceSink, OutputSink> AdapterBoundary<ResourceSink, OutputSink> {
    /// Creates an adapter boundary from host-owned sinks.
    pub fn new(resource_sink: ResourceSink, output_sink: OutputSink) -> Self {
        Self {
            resource_sink,
            output_sink,
        }
    }

    /// Consumes a transaction result by applying plans, then emitting frames.
    pub fn apply_transaction<C, O>(
        &mut self,
        result: TransactionResult<C, O>,
    ) -> Result<AdapterReceipt, AdapterError<ResourceSink::Error, OutputSink::Error>>
    where
        ResourceSink: ResourceCommandSink<C>,
        OutputSink: OutputFrameSink<O>,
    {
        let trace = result.trace();
        let transaction_id = result.transaction_id;
        let revision = result.revision;
        let commands = result.resource_plan.into_commands();
        let output_frames = result.output_frames;
        let resource_command_count = commands.len();
        let output_frame_count = output_frames.len();

        for command in commands {
            self.resource_sink
                .apply(command)
                .map_err(AdapterError::Resource)?;
        }
        for frame in output_frames {
            self.output_sink.emit(frame).map_err(AdapterError::Output)?;
        }

        Ok(AdapterReceipt {
            transaction_id,
            revision,
            resource_command_count,
            output_frame_count,
            trace,
        })
    }

    /// Returns the resource sink.
    pub fn resource_sink(&self) -> &ResourceSink {
        &self.resource_sink
    }

    /// Returns the output sink.
    pub fn output_sink(&self) -> &OutputSink {
        &self.output_sink
    }

    /// Consumes the boundary into its sinks.
    pub fn into_sinks(self) -> (ResourceSink, OutputSink) {
        (self.resource_sink, self.output_sink)
    }
}

/// In-memory resource-command sink for adapter tests and examples.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordingResourceSink<C> {
    commands: Vec<ResourceCommand<C>>,
}

impl<C> RecordingResourceSink<C> {
    /// Returns recorded commands in adapter application order.
    pub fn commands(&self) -> &[ResourceCommand<C>] {
        &self.commands
    }
}

impl<C> Default for RecordingResourceSink<C> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl<C> ResourceCommandSink<C> for RecordingResourceSink<C> {
    type Error = Infallible;

    fn apply(&mut self, command: ResourceCommand<C>) -> Result<(), Self::Error> {
        self.commands.push(command);
        Ok(())
    }
}

/// In-memory output-frame sink for adapter tests and examples.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordingOutputSink<O> {
    frames: Vec<OutputFrame<O>>,
}

impl<O> RecordingOutputSink<O> {
    /// Returns recorded frames in adapter emission order.
    pub fn frames(&self) -> &[OutputFrame<O>] {
        &self.frames
    }
}

impl<O> Default for RecordingOutputSink<O> {
    fn default() -> Self {
        Self { frames: Vec::new() }
    }
}

impl<O> OutputFrameSink<O> for RecordingOutputSink<O> {
    type Error = Infallible;

    fn emit(&mut self, frame: OutputFrame<O>) -> Result<(), Self::Error> {
        self.frames.push(frame);
        Ok(())
    }
}

/// Recording adapter boundary for tests and examples.
pub type RecordingAdapter<C, O> = AdapterBoundary<RecordingResourceSink<C>, RecordingOutputSink<O>>;

impl<C, O> Default for RecordingAdapter<C, O> {
    fn default() -> Self {
        Self::new(
            RecordingResourceSink::default(),
            RecordingOutputSink::default(),
        )
    }
}
