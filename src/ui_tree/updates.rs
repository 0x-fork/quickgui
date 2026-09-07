use super::*;
use crate::ElementUpdate;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ElementUpdateKind {
    #[default]
    None,
    Paint,
    Layout,
}

impl UiTree {
    /// Apply a batch without re-declaring siblings or replacing event registrations. All targets
    /// are validated first; a missing target or callback-owned subtree rejects the whole batch.
    pub(crate) fn update_elements(
        &mut self,
        updates: &[ElementUpdate],
    ) -> Result<Option<ElementUpdateKind>, UiError> {
        let Some(root) = self.root.as_ref() else {
            return Ok(None);
        };
        let mut paths = Vec::with_capacity(updates.len());
        for update in updates {
            let mut id = update.id();
            let mut path = Vec::new();
            while id != root.runtime_id {
                let Some(&(parent, index)) = self.layout_nodes.positions.get(&id) else {
                    return Ok(None);
                };
                path.push(index);
                id = parent;
            }
            path.reverse();
            let mut element = root;
            for index in path.iter().copied().map(Some).chain(std::iter::once(None)) {
                // A later callback evaluation would overwrite an imperative change. Let the
                // source view re-declare these subtrees with its updated state instead.
                if element.animation.is_some()
                    || element.spring.is_some()
                    || element.resolved_motion
                    || matches!(element.kind, ElementKind::ContainerQuery(_))
                {
                    return Ok(None);
                }
                if let Some(index) = index {
                    let Some(child) = element.children.get(index) else {
                        return Ok(None);
                    };
                    element = child;
                }
            }
            if element.runtime_id != update.id()
                || (matches!(update, ElementUpdate::Text { .. })
                    && !matches!(element.kind, ElementKind::Text(_)))
            {
                return Ok(None);
            }
            paths.push(path);
        }

        let mut kind = ElementUpdateKind::None;
        for (update, path) in updates.iter().zip(paths) {
            let mut element = self.root.as_mut().unwrap();
            for index in path {
                element = &mut element.children[index];
            }
            let changed = match update {
                ElementUpdate::Text { id, content } => {
                    let ElementKind::Text(previous) = &mut element.kind else {
                        unreachable!("batch targets were validated before mutation")
                    };
                    if previous == content {
                        continue;
                    }
                    *previous = content.clone();
                    let node = element.taffy_node.unwrap();
                    if let Some(MeasureContext::Text {
                        content: measured, ..
                    }) = self.taffy.get_node_context_mut(node)
                    {
                        *measured = content.clone();
                    }
                    self.taffy.mark_dirty(node)?;
                    if let Some(&index) = self.selectable_text_indices.get(id) {
                        let entry = &mut self.selectable_texts[index];
                        entry.content = content.clone();
                        entry.character_lengths = selectable_character_lengths(content).into();
                    }
                    self.retained_semantics_dirty = true;
                    kind = ElementUpdateKind::Layout;
                    true
                }
                ElementUpdate::BackgroundColor { color, .. } => {
                    if element.visual.background == Some(*color) {
                        false
                    } else {
                        element.visual.background = Some(*color);
                        true
                    }
                }
                ElementUpdate::TextColor { color, .. } => {
                    if element.typography.color == Some(*color) {
                        false
                    } else {
                        element.typography.color = Some(*color);
                        update_inherited_color(element, *color, &mut self.taffy);
                        true
                    }
                }
                ElementUpdate::Opacity { opacity, .. } => {
                    let opacity = if opacity.is_finite() {
                        opacity.clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    if element.visual.opacity == opacity {
                        false
                    } else {
                        element.visual.opacity = opacity;
                        true
                    }
                }
            };
            if changed && kind == ElementUpdateKind::None {
                kind = ElementUpdateKind::Paint;
            }
        }
        if kind == ElementUpdateKind::Layout {
            sync_static_text_selection(
                &mut self.static_text_selection,
                &self.selectable_texts,
                &self.selectable_text_indices,
            );
        }
        Ok(Some(kind))
    }

    pub(crate) fn take_retained_semantics_dirty(&mut self) -> bool {
        std::mem::take(&mut self.retained_semantics_dirty)
    }
}

fn update_inherited_color(
    element: &mut Element,
    color: Color,
    taffy: &mut TaffyTree<MeasureContext>,
) {
    element.resolved_typography.color = color;
    if let Some(node) = element.taffy_node
        && let Some(MeasureContext::Text { style, .. }) = taffy.get_node_context_mut(node)
    {
        style.color = color;
    }
    for child in &mut element.children {
        // An explicitly colored child is an inheritance boundary for its complete subtree.
        if child.typography.color.is_none() {
            update_inherited_color(child, color, taffy);
        }
    }
}
