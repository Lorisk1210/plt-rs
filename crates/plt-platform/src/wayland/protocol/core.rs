//! The core protocol (`wayland.xml`): the display and registry, surfaces
//! and regions, subsurfaces, shared-memory buffers and outputs.

use std::os::fd::OwnedFd;

use super::{Event, Interface};
use crate::error::Error;
use crate::wayland::wire::{Args, Request};

/// The id of the `wl_display` singleton.
pub(crate) const DISPLAY: u32 = 1;

/// `wl_shm.format.xrgb8888`: opaque pixels, `0xXXRRGGBB` words.
pub(crate) const FORMAT_XRGB8888: u32 = 1;
/// `wl_shm.format.argb8888`: premultiplied `0xAARRGGBB` words.
pub(crate) const FORMAT_ARGB8888: u32 = 0;

/// `wl_display.sync`: `callback` is done once every earlier request
/// was handled.
pub(crate) fn sync(callback: u32) -> Request {
    Request::new(DISPLAY, 0).uint(callback)
}

/// `wl_display.get_registry`.
pub(crate) fn get_registry(registry: u32) -> Request {
    Request::new(DISPLAY, 1).uint(registry)
}

/// `wl_registry.bind`: binds global `name` of `interface` at `version`
/// to the new object `id`.
pub(crate) fn bind(registry: u32, name: u32, interface: &str, version: u32, id: u32) -> Request {
    Request::new(registry, 0)
        .uint(name)
        .string(interface)
        .uint(version)
        .uint(id)
}

/// `wl_compositor.create_surface`.
pub(crate) fn create_surface(compositor: u32, surface: u32) -> Request {
    Request::new(compositor, 0).uint(surface)
}

/// `wl_surface.destroy`.
pub(crate) fn destroy_surface(surface: u32) -> Request {
    Request::new(surface, 0)
}

/// `wl_surface.attach` at offset 0 (`buffer` 0 unmaps the surface).
pub(crate) fn attach(surface: u32, buffer: u32) -> Request {
    Request::new(surface, 1).uint(buffer).int(0).int(0)
}

/// `wl_surface.frame`: `callback` is done when it is a good time to draw
/// the next frame.
pub(crate) fn frame(surface: u32, callback: u32) -> Request {
    Request::new(surface, 3).uint(callback)
}

/// `wl_surface.commit`.
pub(crate) fn commit(surface: u32) -> Request {
    Request::new(surface, 6)
}

/// `wl_surface.set_buffer_scale` (version 3).
pub(crate) fn set_buffer_scale(surface: u32, scale: i32) -> Request {
    Request::new(surface, 8).int(scale)
}

/// `wl_surface.damage_buffer` (version 4), in buffer pixels.
pub(crate) fn damage_buffer(surface: u32, x: i32, y: i32, width: i32, height: i32) -> Request {
    Request::new(surface, 9)
        .int(x)
        .int(y)
        .int(width)
        .int(height)
}

/// `wl_subcompositor.get_subsurface`: makes `surface` a child of
/// `parent`.
pub(crate) fn get_subsurface(
    subcompositor: u32,
    subsurface: u32,
    surface: u32,
    parent: u32,
) -> Request {
    Request::new(subcompositor, 1)
        .uint(subsurface)
        .uint(surface)
        .uint(parent)
}

/// `wl_subsurface.destroy`.
pub(crate) fn destroy_subsurface(subsurface: u32) -> Request {
    Request::new(subsurface, 0)
}

/// `wl_subsurface.set_position`, relative to the parent.
pub(crate) fn set_position(subsurface: u32, x: i32, y: i32) -> Request {
    Request::new(subsurface, 1).int(x).int(y)
}

/// `wl_subsurface.place_below`: stacks the subsurface under `sibling`
/// (or its parent).
pub(crate) fn place_below(subsurface: u32, sibling: u32) -> Request {
    Request::new(subsurface, 3).uint(sibling)
}

/// `wl_shm.create_pool` over the memory of `fd`.
pub(crate) fn create_pool(shm: u32, pool: u32, fd: OwnedFd, size: i32) -> Request {
    Request::new(shm, 0).uint(pool).fd(fd).int(size)
}

/// `wl_shm_pool.create_buffer` of 32-bit pixels at `offset`.
pub(crate) fn create_buffer(
    pool: u32,
    buffer: u32,
    offset: i32,
    (width, height): (i32, i32),
    format: u32,
) -> Request {
    Request::new(pool, 0)
        .uint(buffer)
        .int(offset)
        .int(width)
        .int(height)
        .int(width.saturating_mul(4))
        .uint(format)
}

/// `wl_shm_pool.destroy` (buffers created from it stay valid).
pub(crate) fn destroy_pool(pool: u32) -> Request {
    Request::new(pool, 1)
}

/// `wl_buffer.destroy`.
pub(crate) fn destroy_buffer(buffer: u32) -> Request {
    Request::new(buffer, 0)
}

/// `wl_output.release` (version 3).
pub(crate) fn release_output(output: u32) -> Request {
    Request::new(output, 0)
}

/// Parses the events of the core interfaces that have any.
pub(super) fn parse(
    interface: Interface,
    opcode: u16,
    args: &mut Args<'_>,
) -> Result<Event, Error> {
    Ok(match (interface, opcode) {
        (Interface::Display, 0) => Event::Error {
            object: args.uint()?,
            code: args.uint()?,
            message: args.string()?,
        },
        (Interface::Display, 1) => Event::DeleteId(args.uint()?),
        (Interface::Registry, 0) => Event::Global {
            name: args.uint()?,
            interface: args.string()?,
            version: args.uint()?,
        },
        (Interface::Registry, 1) => Event::GlobalRemove(args.uint()?),
        (Interface::Callback, 0) => Event::Done,
        (Interface::Buffer, 0) => Event::Release,
        (Interface::Surface, 0) => Event::SurfaceEnter(args.uint()?),
        (Interface::Surface, 1) => Event::SurfaceLeave(args.uint()?),
        (Interface::Surface, 2) => Event::PreferredBufferScale(args.int()?),
        (Interface::Output, 3) => Event::OutputScale(args.int()?),
        _ => Event::Ignored,
    })
}
