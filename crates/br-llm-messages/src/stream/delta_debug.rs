use std::fmt;

use super::delta::DeltaOp;

struct Redacted;

impl fmt::Debug for Redacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl fmt::Debug for DeltaOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeltaOp::ResponseStarted { metadata } => f
                .debug_struct("ResponseStarted")
                .field("metadata", metadata)
                .finish(),
            DeltaOp::BlockStarted { block, start } => f
                .debug_struct("BlockStarted")
                .field("block", block)
                .field("start", start)
                .finish(),
            DeltaOp::TextAppended { block, text } => f
                .debug_struct("TextAppended")
                .field("block", block)
                .field("text", text)
                .finish(),
            DeltaOp::ThinkingAppended { block, text } => f
                .debug_struct("ThinkingAppended")
                .field("block", block)
                .field("text", text)
                .finish(),
            DeltaOp::JsonAppended { block, .. } => f
                .debug_struct("JsonAppended")
                .field("block", block)
                .field("fragment", &Redacted)
                .finish(),
            DeltaOp::ReplayFragmentAppended { block, format, .. } => f
                .debug_struct("ReplayFragmentAppended")
                .field("block", block)
                .field("format", format)
                .field("fragment", &Redacted)
                .finish(),
            DeltaOp::ReplayMaterialSet { block, material } => f
                .debug_struct("ReplayMaterialSet")
                .field("block", block)
                .field("material", material)
                .finish(),
            DeltaOp::BlockMetadataUpdated { block, update } => f
                .debug_struct("BlockMetadataUpdated")
                .field("block", block)
                .field("update", update)
                .finish(),
            DeltaOp::BlockInserted { block } => f
                .debug_struct("BlockInserted")
                .field("block", block)
                .finish(),
            DeltaOp::BlockFinalized {
                block,
                finalization,
            } => f
                .debug_struct("BlockFinalized")
                .field("block", block)
                .field("finalization", finalization)
                .finish(),
            DeltaOp::ResponseMetadataUpdated { update } => f
                .debug_struct("ResponseMetadataUpdated")
                .field("update", update)
                .finish(),
            DeltaOp::ResponseCompleted { ending } => f
                .debug_struct("ResponseCompleted")
                .field("ending", ending)
                .finish(),
            DeltaOp::ResponseInterrupted { reason } => f
                .debug_struct("ResponseInterrupted")
                .field("reason", reason)
                .finish(),
        }
    }
}
