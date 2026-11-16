use std::ops::Range;
use std::result;
use std::sync::Arc;

use anyhow::Result;
use futures::future::Shared;
use gpui::App;
use gpui::Entity;
use gpui::Task;
use gpui::WeakEntity;
use language::Buffer;
use language::BufferRow;
use language::CharKind;
use lsp::LanguageServerId;
use project::DocumentHighlight;
use project::InvalidationStrategy;
use project::Location;
use project::LocationLink;
use project::PrepareRenameResponse;
use project::Project;
use project::ProjectTransaction;
use project::lsp_store::BufferSemanticTokens;
use project::lsp_store::CacheInlayHints;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use text::BufferId;

use super::GotoDefinitionKind;
use super::RenameTarget;

pub trait SemanticsProvider {
    fn hover(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        cx: &mut App,
    ) -> Option<Task<Option<Vec<project::Hover>>>>;

    fn applicable_inlay_chunks(
        &self,
        buffer: &Entity<Buffer>,
        ranges: &[Range<text::Anchor>],
        cx: &mut App,
    ) -> Vec<Range<BufferRow>>;

    fn invalidate_inlay_hints(&self, for_buffers: &FxHashSet<BufferId>, cx: &mut App);

    fn inlay_hints(
        &self,
        invalidate: InvalidationStrategy,
        buffer: Entity<Buffer>,
        ranges: Vec<Range<text::Anchor>>,
        known_chunks: Option<(clock::Global, FxHashSet<Range<BufferRow>>)>,
        cx: &mut App,
    ) -> Option<FxHashMap<Range<BufferRow>, Task<Result<CacheInlayHints>>>>;

    #[allow(clippy::type_complexity)]
    fn semantic_tokens(
        &self,
        buffer: Entity<Buffer>,
        cx: &mut App,
    ) -> Option<Shared<Task<result::Result<BufferSemanticTokens, Arc<anyhow::Error>>>>>;

    fn supports_inlay_hints(&self, buffer: &Entity<Buffer>, cx: &mut App) -> bool;

    fn supports_semantic_tokens(&self, buffer: &Entity<Buffer>, cx: &mut App) -> bool;

    fn supports_document_highlights(&self, buffer: &Entity<Buffer>, cx: &mut App) -> bool;

    fn document_highlights(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        cx: &mut App,
    ) -> Option<Task<Result<Vec<DocumentHighlight>>>>;

    fn definitions(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        kind: GotoDefinitionKind,
        cx: &mut App,
    ) -> Option<Task<Result<Option<Vec<LocationLink>>>>>;

    fn references(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        cx: &mut App,
    ) -> Option<Task<Result<Option<Vec<Location>>>>>;

    fn range_for_rename(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        cx: &mut App,
    ) -> Task<Result<Option<RenameTarget>>>;

    fn perform_rename(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        new_name: String,
        language_server_id: Option<LanguageServerId>,
        cx: &mut App,
    ) -> Option<Task<Result<ProjectTransaction>>>;
}

impl SemanticsProvider for WeakEntity<Project> {
    fn hover(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        cx: &mut App,
    ) -> Option<Task<Option<Vec<project::Hover>>>> {
        self.update(cx, |project, cx| project.hover(buffer, position, cx)).ok()
    }

    fn document_highlights(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        cx: &mut App,
    ) -> Option<Task<Result<Vec<DocumentHighlight>>>> {
        self.update(cx, |project, cx| project.document_highlights(buffer, position, cx)).ok()
    }

    fn definitions(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        kind: GotoDefinitionKind,
        cx: &mut App,
    ) -> Option<Task<Result<Option<Vec<LocationLink>>>>> {
        self.update(cx, |project, cx| match kind {
            GotoDefinitionKind::Symbol => project.definitions(buffer, position, cx),
            GotoDefinitionKind::Declaration => project.declarations(buffer, position, cx),
            GotoDefinitionKind::Type => project.type_definitions(buffer, position, cx),
            GotoDefinitionKind::Implementation => project.implementations(buffer, position, cx),
        })
        .ok()
    }

    fn references(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        cx: &mut App,
    ) -> Option<Task<Result<Option<Vec<Location>>>>> {
        self.update(cx, |project, cx| project.references(buffer, position, cx)).ok()
    }

    fn supports_inlay_hints(&self, buffer: &Entity<Buffer>, cx: &mut App) -> bool {
        self.update(cx, |project, cx| {
            buffer.update(cx, |buffer, cx| project.any_language_server_supports_inlay_hints(buffer, cx))
        })
        .unwrap_or(false)
    }

    fn supports_semantic_tokens(&self, buffer: &Entity<Buffer>, cx: &mut App) -> bool {
        self.update(cx, |project, cx| {
            buffer.update(cx, |buffer, cx| project.any_language_server_supports_semantic_tokens(buffer, cx))
        })
        .unwrap_or(false)
    }

    fn supports_document_highlights(&self, buffer: &Entity<Buffer>, cx: &mut App) -> bool {
        self.update(cx, |project, cx| {
            buffer.update(cx, |buffer, cx| project.any_language_server_supports_document_highlights(buffer, cx))
        })
        .unwrap_or(false)
    }

    fn applicable_inlay_chunks(
        &self,
        buffer: &Entity<Buffer>,
        ranges: &[Range<text::Anchor>],
        cx: &mut App,
    ) -> Vec<Range<BufferRow>> {
        self.update(cx, |project, cx| {
            project.lsp_store().update(cx, |lsp_store, cx| lsp_store.applicable_inlay_chunks(buffer, ranges, cx))
        })
        .unwrap_or_default()
    }

    fn invalidate_inlay_hints(&self, for_buffers: &FxHashSet<BufferId>, cx: &mut App) {
        self.update(cx, |project, cx| {
            project.lsp_store().update(cx, |lsp_store, _| lsp_store.invalidate_inlay_hints(for_buffers))
        })
        .ok();
    }

    fn inlay_hints(
        &self,
        invalidate: InvalidationStrategy,
        buffer: Entity<Buffer>,
        ranges: Vec<Range<text::Anchor>>,
        known_chunks: Option<(clock::Global, FxHashSet<Range<BufferRow>>)>,
        cx: &mut App,
    ) -> Option<FxHashMap<Range<BufferRow>, Task<Result<CacheInlayHints>>>> {
        self.update(cx, |project, cx| {
            project
                .lsp_store()
                .update(cx, |lsp_store, cx| lsp_store.inlay_hints(invalidate, buffer, ranges, known_chunks, cx))
        })
        .ok()
    }

    fn semantic_tokens(
        &self,
        buffer: Entity<Buffer>,
        cx: &mut App,
    ) -> Option<Shared<Task<result::Result<BufferSemanticTokens, Arc<anyhow::Error>>>>> {
        self.update(cx, |this, cx| this.lsp_store().update(cx, |lsp_store, cx| lsp_store.semantic_tokens(buffer, cx)))
            .ok()
    }

    fn range_for_rename(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        cx: &mut App,
    ) -> Task<Result<Option<RenameTarget>>> {
        let Some(this) = self.upgrade() else {
            return Task::ready(Ok(None));
        };

        this.update(cx, |project, cx| {
            let buffer = buffer.clone();
            let task = project.prepare_rename(buffer.clone(), position, cx);
            cx.spawn(async move |_, cx| {
                Ok(match task.await? {
                    PrepareRenameResponse::Success { range, language_server_id } => {
                        Some(RenameTarget { range, language_server_id })
                    }
                    PrepareRenameResponse::InvalidPosition => None,
                    PrepareRenameResponse::OnlyUnpreparedRenameSupported => {
                        // Fallback on using TreeSitter info to determine identifier range
                        buffer.read_with(cx, |buffer, _| {
                            let snapshot = buffer.snapshot();
                            let (range, kind) = snapshot.surrounding_word(position, None);
                            if kind != Some(CharKind::Word) {
                                return None;
                            }
                            Some(RenameTarget {
                                range: snapshot.anchor_before(range.start)..snapshot.anchor_after(range.end),
                                language_server_id: None,
                            })
                        })
                    }
                })
            })
        })
    }

    fn perform_rename(
        &self,
        buffer: &Entity<Buffer>,
        position: text::Anchor,
        new_name: String,
        language_server_id: Option<LanguageServerId>,
        cx: &mut App,
    ) -> Option<Task<Result<ProjectTransaction>>> {
        self.update(cx, |project, cx| {
            project.perform_rename(buffer.clone(), position, new_name, language_server_id, cx)
        })
        .ok()
    }
}
