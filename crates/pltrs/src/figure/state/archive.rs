//! [`Archive`] for a figure's state: its artists, removed ones included,
//! under their ids, and what lays them out. What only lives while a
//! figure is shown (canvas, callbacks, timers, tools, the mouse grab and
//! widget lock, problems pending report) is not archived: a figure read
//! back starts without it, stale, as a new figure does.

use std::cell::Cell;

use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder};

use super::{FigureState, StateRole};
use crate::artist::{ArtistId, ArtistStore, DetachedArtists};
use crate::axes::{LimitCallbacks, ShareGroups};
use crate::events::{CallbackRegistry, CursorShape, TimerRegistry};
use crate::figure::{CanvasState, FigureFrame, FreshArtists};
use crate::gui::tools::ToolRegistry;
use crate::layout::LayoutState;

pltrs_core::archive_enum!(StateRole {
    Figure,
    LooseHolder
});

impl Archive for FigureState {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        let Self {
            store,
            detached,
            root,
            dpi,
            original_dpi,
            size_inches,
            stale: _,
            fresh: _,
            pending_errors: _,
            shared,
            layout,
            current_axes,
            limit_callbacks: _,
            callbacks: _,
            timers: _,
            mouse_grabber: _,
            capture_keys: _,
            widget_lock: _,
            cursor: _,
            tools: _,
            print_window: _,
            saving: _,
            suppress_composite,
            last_draw_dpi,
            canvas: _,
            draw_hook: _,
            layout_hook: _,
            tick_updater: _,
            draws_per_layout: _,
            role,
        } = self;
        store.encode(encoder)?;
        detached.encode(encoder)?;
        root.encode(encoder)?;
        dpi.encode(encoder)?;
        original_dpi.encode(encoder)?;
        size_inches.encode(encoder)?;
        shared.encode(encoder)?;
        layout.encode(encoder)?;
        current_axes.encode(encoder)?;
        suppress_composite.encode(encoder)?;
        last_draw_dpi.encode(encoder)?;
        role.encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let store = ArtistStore::decode(decoder)?;
        let detached = DetachedArtists::decode(decoder)?;
        let root = ArtistId::decode(decoder)?;
        let dpi = Cell::decode(decoder)?;
        let state = Self {
            store,
            detached,
            root,
            dpi,
            original_dpi: Archive::decode(decoder)?,
            size_inches: Archive::decode(decoder)?,
            stale: true,
            fresh: FreshArtists::default(),
            pending_errors: Vec::new(),
            shared: ShareGroups::decode(decoder)?,
            layout: LayoutState::decode(decoder)?,
            current_axes: Archive::decode(decoder)?,
            limit_callbacks: LimitCallbacks::default(),
            callbacks: CallbackRegistry::default(),
            timers: TimerRegistry::default(),
            mouse_grabber: None,
            capture_keys: false,
            widget_lock: None,
            cursor: CursorShape::default(),
            tools: ToolRegistry::default(),
            print_window: None,
            saving: false,
            suppress_composite: Archive::decode(decoder)?,
            last_draw_dpi: Archive::decode(decoder)?,
            canvas: CanvasState::default(),
            draw_hook: None,
            layout_hook: None,
            tick_updater: None,
            draws_per_layout: 1,
            role: StateRole::decode(decoder)?,
        };
        if state.detached.overlaps(&state.store) {
            return Err(ArchiveError::invalid("removed artists still in the figure"));
        }
        if state.store.get_as::<FigureFrame>(state.root).is_none() {
            return Err(ArchiveError::invalid("the figure has no root frame"));
        }
        Ok(state)
    }
}
