//! Routing received events to the objects they concern, and the events of
//! the registry, outputs, windows and scales.

use super::connection::{Received, WAKE_CALLBACK};
use super::globals::{self, OUTPUT_VERSION, SEAT_VERSION};
use super::protocol::core;
use super::protocol::shell;
use super::protocol::{Event, Interface};
use super::seat::Seat;
use super::{Output, WaylandBackend, emit};
use crate::error::Error;
use crate::event::{Event as LoopEvent, WindowEvent};

impl WaylandBackend {
    /// Handles received events in order, appending what they mean for the
    /// application to `out`.
    pub(super) fn dispatch(
        &mut self,
        events: Vec<Received>,
        out: &mut Vec<LoopEvent>,
    ) -> Result<(), Error> {
        for Received {
            object,
            interface,
            event,
        } in events
        {
            match interface {
                Interface::Registry => self.registry_event(event)?,
                Interface::Callback => self.callback_done(object, out)?,
                Interface::WmBase => {
                    if let Event::Ping(serial) = event {
                        self.conn.send(shell::pong(object, serial))?;
                    }
                }
                Interface::Buffer => self.buffer_released(object)?,
                Interface::Output => {
                    if let Event::OutputScale(scale) = event {
                        self.output_scale(object, scale, out)?;
                    }
                }
                Interface::Seat => {
                    if let Event::Capabilities(capabilities) = event {
                        self.seat_capabilities(capabilities)?;
                    }
                }
                Interface::Pointer => self.pointer_event(&event, out)?,
                Interface::Keyboard => self.keyboard_event(event, out),
                Interface::DataDevice | Interface::DataOffer | Interface::DataSource => {
                    self.clipboard_event(object, event)?;
                }
                Interface::ActivationToken => {
                    if let Event::ActivationToken(token) = event {
                        self.activate(object, &token)?;
                    }
                }
                _ => self.window_event(object, event, out)?,
            }
        }
        Ok(())
    }

    fn registry_event(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Event::Global {
                name,
                interface,
                version,
            } => {
                let registry = self.registry;
                match interface.as_str() {
                    "wl_output" => {
                        let version = version.min(OUTPUT_VERSION);
                        let global = (name, interface.as_str(), version);
                        let id =
                            globals::bind(&mut self.conn, registry, Interface::Output, global)?;
                        self.outputs.insert(
                            id,
                            Output {
                                name,
                                version,
                                scale: 1,
                            },
                        );
                    }
                    "wl_seat" if self.seat.is_none() => {
                        let version = version.min(SEAT_VERSION);
                        let global = (name, interface.as_str(), version);
                        let id = globals::bind(&mut self.conn, registry, Interface::Seat, global)?;
                        self.seat = Some(Seat::new(id, name, version));
                    }
                    _ => {
                        self.globals
                            .bind(&mut self.conn, registry, (name, &interface, version))?;
                    }
                }
                self.ensure_data_device()
            }
            Event::GlobalRemove(name) => self.global_removed(name),
            _ => Ok(()),
        }
    }

    fn global_removed(&mut self, name: u32) -> Result<(), Error> {
        if let Some((&id, output)) = self.outputs.iter().find(|(_, output)| output.name == name) {
            if output.version >= 3 {
                self.conn.send(core::release_output(id))?;
            }
            self.outputs.remove(&id);
            for window in self.windows.values_mut() {
                window.scale_sources.outputs.retain(|&output| output != id);
            }
        } else if self.seat.as_ref().is_some_and(|seat| seat.name == name) {
            self.seat_capabilities(0)?;
            self.seat = None;
        } else {
            self.globals.remove(name);
        }
        Ok(())
    }

    /// A callback is done: a wake-up, or a window's frame shown.
    fn callback_done(&mut self, callback: u32, out: &mut Vec<LoopEvent>) -> Result<(), Error> {
        if callback == WAKE_CALLBACK {
            if self.waker.accept() && !out.contains(&LoopEvent::WakeUp) {
                out.push(LoopEvent::WakeUp);
            }
            return Ok(());
        }
        let window = self
            .windows
            .iter_mut()
            .find_map(|(&id, window)| window.frame_done(callback).then_some(id));
        match window {
            Some(window) => self.commit_window(window, false),
            None => Ok(()),
        }
    }

    fn buffer_released(&mut self, buffer: u32) -> Result<(), Error> {
        let window = self
            .windows
            .iter_mut()
            .find_map(|(&id, window)| window.release(buffer).then_some(id));
        match window {
            Some(window) => self.commit_window(window, false),
            None => Ok(()),
        }
    }

    fn output_scale(
        &mut self,
        output: u32,
        scale: i32,
        out: &mut Vec<LoopEvent>,
    ) -> Result<(), Error> {
        if let Some(state) = self.outputs.get_mut(&output) {
            state.scale = u32::try_from(scale).unwrap_or(1).clamp(1, 16);
        }
        let windows: Vec<u32> = self.windows.keys().copied().collect();
        for window in windows {
            self.update_scale(window, out)?;
        }
        Ok(())
    }

    /// Events of a window's own objects.
    fn window_event(
        &mut self,
        object: u32,
        event: Event,
        out: &mut Vec<LoopEvent>,
    ) -> Result<(), Error> {
        let Some((&id, window)) = self.windows.iter_mut().find(|(_, window)| {
            [
                Some(window.surface),
                Some(window.xdg_surface),
                Some(window.toplevel),
            ]
            .contains(&Some(object))
                || window.server_decoration == Some(object)
                || window.fractional_scale == Some(object)
        }) else {
            return Ok(());
        };
        match event {
            Event::SurfaceEnter(output) => {
                if self.outputs.contains_key(&output)
                    && !window.scale_sources.outputs.contains(&output)
                {
                    window.scale_sources.outputs.push(output);
                }
                self.update_scale(id, out)
            }
            Event::SurfaceLeave(output) => {
                window
                    .scale_sources
                    .outputs
                    .retain(|&entered| entered != output);
                self.update_scale(id, out)
            }
            Event::PreferredBufferScale(scale) => {
                window.scale_sources.preferred = u32::try_from(scale).ok();
                self.update_scale(id, out)
            }
            Event::PreferredScale(scale) => {
                window.scale_sources.fractional = Some(scale);
                self.update_scale(id, out)
            }
            Event::ToplevelConfigure {
                width,
                height,
                states,
            } => {
                window.toplevel_configure(width, height, &states);
                Ok(())
            }
            Event::WmCapabilities(capabilities) => {
                window.wm_capabilities(&capabilities);
                Ok(())
            }
            Event::DecorationMode(mode) => {
                window.decoration_mode(mode);
                Ok(())
            }
            Event::Close => {
                emit(out, id, WindowEvent::CloseRequested);
                Ok(())
            }
            Event::Configure(serial) => {
                let configured = window.configure(&mut self.conn, &self.globals, serial)?;
                if configured.first {
                    emit(out, id, WindowEvent::RedrawRequested);
                }
                if configured.resized {
                    emit(out, id, WindowEvent::Resized(window.size()));
                }
                window.request_commit();
                self.commit_window(id, false)
            }
            _ => Ok(()),
        }
    }

    /// Recomputes a window's scale from its sources, reporting changes
    /// once it is shown.
    pub(super) fn update_scale(&mut self, id: u32, out: &mut Vec<LoopEvent>) -> Result<(), Error> {
        let fractional = self.fractional_scaling();
        let fallback = self.largest_output_scale();
        let outputs = &self.outputs;
        let Some(window) = self.windows.get_mut(&id) else {
            return Ok(());
        };
        let scale = window.scale_sources.scale(
            fractional,
            |output| outputs.get(&output).map(|o| o.scale),
            fallback,
        );
        let old_size = window.size();
        if window.set_scale(&mut self.conn, scale)? && window.configured {
            emit(out, id, WindowEvent::ScaleFactorChanged(scale.factor()));
            if window.size() != old_size {
                emit(out, id, WindowEvent::Resized(window.size()));
            }
            window.request_commit();
        }
        Ok(())
    }

    /// Answers an activation token: focus its surface.
    fn activate(&mut self, token: u32, value: &str) -> Result<(), Error> {
        self.conn.send(shell::destroy_token(token))?;
        if let (Some(surface), Some(activation)) = (
            self.activations.remove(&token),
            self.globals.get(Interface::Activation),
        ) && self.windows.contains_key(&surface)
        {
            self.conn
                .send(shell::activate(activation, value, surface))?;
        }
        Ok(())
    }
}
