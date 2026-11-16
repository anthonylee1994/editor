use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use gpui::AnyEntity;
use gpui::AnyView;
use gpui::App;
use gpui::AppContext;
use gpui::Context;
use gpui::Entity;
use gpui::Global;
use gpui::Task;
use gpui::WeakEntity;
use gpui_util::TypeIdHashBuilder;
use project::Project;
use project::ProjectEntryId;
use project::ProjectPath;
use rustc_hash::FxHashMap;
use ui::Window;

use super::Pane;
use super::Workspace;
use crate::item::ItemHandle;
use crate::item::ProjectItem;
use crate::item::SerializableItem;
use crate::item::SerializableItemHandle;

type BuildProjectItemFn = fn(AnyEntity, Entity<Project>, Option<&Pane>, &mut Window, &mut App) -> Box<dyn ItemHandle>;

type BuildProjectItemForPathFn = fn(
    &Entity<Project>,
    &ProjectPath,
    &mut Window,
    &mut App,
) -> Option<Task<Result<(Option<ProjectEntryId>, WorkspaceItemBuilder)>>>;

#[derive(Clone, Default)]
pub(super) struct ProjectItemRegistry {
    build_project_item_fns_by_type: HashMap<TypeId, BuildProjectItemFn, TypeIdHashBuilder>,
    build_project_item_for_path_fns: Vec<BuildProjectItemForPathFn>,
}

impl ProjectItemRegistry {
    fn register<T: ProjectItem>(&mut self) {
        self.build_project_item_fns_by_type.insert(TypeId::of::<T::Item>(), |item, project, pane, window, cx| {
            let item = item.downcast().unwrap();
            Box::new(cx.new(|cx| T::for_project_item(project, pane, item, window, cx))) as Box<dyn ItemHandle>
        });
        self.build_project_item_for_path_fns.push(|project, project_path, window, cx| {
            let project_path = project_path.clone();
            let is_file = project.read(cx).entry_for_path(&project_path, cx).is_some_and(|entry| entry.is_file());
            let entry_abs_path = project.read(cx).absolute_path(&project_path, cx);
            let project_item = <T::Item as project::ProjectItem>::try_open(project, &project_path, cx)?;
            let project = project.clone();
            Some(window.spawn(cx, async move |cx| {
                match project_item.await.with_context(|| {
                    format!(
                        "opening project path {:?}",
                        entry_abs_path.as_deref().unwrap_or(project_path.path.as_std_path())
                    )
                }) {
                    Ok(project_item) => {
                        let project_item = project_item;
                        let project_entry_id: Option<ProjectEntryId> =
                            project_item.read_with(cx, project::ProjectItem::entry_id);
                        let build_workspace_item =
                            Box::new(|pane: &mut Pane, window: &mut Window, cx: &mut Context<Pane>| {
                                Box::new(
                                    cx.new(|cx| T::for_project_item(project, Some(pane), project_item, window, cx)),
                                ) as Box<dyn ItemHandle>
                            }) as Box<_>;
                        Ok((project_entry_id, build_workspace_item))
                    }
                    Err(e) => {
                        log::warn!("Failed to open a project item: {e:#}");
                        if let Some(abs_path) = entry_abs_path.as_deref().filter(|_| is_file)
                            && let Some(broken_project_item_view) =
                                cx.update(|window, cx| T::for_broken_project_item(abs_path, &e, window, cx))?
                        {
                            let build_workspace_item =
                                Box::new(move |_: &mut Pane, _: &mut Window, cx: &mut Context<Pane>| {
                                    cx.new(|_| broken_project_item_view).boxed_clone()
                                }) as Box<_>;
                            return Ok((None, build_workspace_item));
                        }
                        Err(e)
                    }
                }
            }))
        });
    }

    pub(super) fn open_path(
        &self,
        project: &Entity<Project>,
        path: &ProjectPath,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<(Option<ProjectEntryId>, WorkspaceItemBuilder)>> {
        let Some(open_project_item) = self
            .build_project_item_for_path_fns
            .iter()
            .rev()
            .find_map(|open_project_item| open_project_item(project, path, window, cx))
        else {
            return Task::ready(Err(anyhow!("cannot open file {:?}", path.path)));
        };
        open_project_item
    }
}

pub(super) type WorkspaceItemBuilder =
    Box<dyn FnOnce(&mut Pane, &mut Window, &mut Context<Pane>) -> Box<dyn ItemHandle>>;

impl Global for ProjectItemRegistry {}

/// Registers a [ProjectItem] for the app. When opening a file, all the registered
/// items will get a chance to open the file, starting from the project item that
/// was added last.
pub fn register_project_item<I: ProjectItem>(cx: &mut App) {
    cx.default_global::<ProjectItemRegistry>().register::<I>();
}

#[derive(Copy, Clone)]
struct SerializableItemDescriptor {
    #[allow(clippy::type_complexity)]
    deserialize: fn(
        Entity<Project>,
        WeakEntity<Workspace>,
        serde_json::Value,
        &mut Window,
        &mut Context<Pane>,
    ) -> Task<Result<Box<dyn ItemHandle>>>,
    view_to_serializable_item: fn(AnyView) -> Box<dyn SerializableItemHandle>,
}

#[derive(Default)]
pub(crate) struct SerializableItemRegistry {
    descriptors_by_kind: FxHashMap<Arc<str>, SerializableItemDescriptor>,
    descriptors_by_type: HashMap<TypeId, SerializableItemDescriptor, TypeIdHashBuilder>,
}

impl Global for SerializableItemRegistry {}

impl SerializableItemRegistry {
    pub(super) fn deserialize(
        item_kind: &str,
        project: Entity<Project>,
        workspace: WeakEntity<Workspace>,
        state: serde_json::Value,
        window: &mut Window,
        cx: &mut Context<Pane>,
    ) -> Task<Result<Box<dyn ItemHandle>>> {
        let Some(descriptor) = Self::descriptor(item_kind, cx) else {
            return Task::ready(Err(anyhow!("cannot deserialize {}, descriptor not found", item_kind)));
        };

        (descriptor.deserialize)(project, workspace, state, window, cx)
    }

    pub(super) fn view_to_serializable_item_handle(view: AnyView, cx: &App) -> Option<Box<dyn SerializableItemHandle>> {
        let this = cx.try_global::<Self>()?;
        let descriptor = this.descriptors_by_type.get(&view.entity_type())?;
        Some((descriptor.view_to_serializable_item)(view))
    }

    fn descriptor(item_kind: &str, cx: &App) -> Option<SerializableItemDescriptor> {
        let this = cx.try_global::<Self>()?;
        this.descriptors_by_kind.get(item_kind).copied()
    }
}

pub fn register_serializable_item<I: SerializableItem>(cx: &mut App) {
    let serialized_item_kind = I::serialized_item_kind();

    let registry = cx.default_global::<SerializableItemRegistry>();
    let descriptor = SerializableItemDescriptor {
        deserialize: |project, workspace, state, window, cx| {
            let task = I::deserialize(project, workspace, state, window, cx);
            cx.foreground_executor().spawn(async { Ok(Box::new(task.await?) as Box<_>) })
        },
        view_to_serializable_item: |view| Box::new(view.downcast::<I>().unwrap()),
    };
    registry.descriptors_by_kind.insert(Arc::from(serialized_item_kind), descriptor);
    registry.descriptors_by_type.insert(TypeId::of::<I>(), descriptor);
}
