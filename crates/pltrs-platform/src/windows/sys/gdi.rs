//! Painting windows, cursors and icons.

use std::ffi::c_void;

use super::ffi::{self, BitmapInfo, BitmapInfoHeader, Handle, PaintStruct};
use super::{Hwnd, last_error};
use crate::error::Error;
use crate::geometry::PhysicalRect;
use crate::window::{CursorIcon, Icon};
use crate::windows::convert::{cursor_resource, icon_pixels, mask_len};

/// A `WM_PAINT` in progress (`BeginPaint`, ended by dropping it with
/// `EndPaint`).
#[derive(Debug)]
pub(crate) struct Painter {
    window: Hwnd,
    paint: PaintStruct,
}

impl Painter {
    /// Starts painting `window`'s invalid part.
    pub(crate) fn begin(window: Hwnd) -> Self {
        let mut paint = PaintStruct::default();
        // SAFETY: `paint` is a PAINTSTRUCT to fill; `Drop` ends the paint.
        unsafe { ffi::BeginPaint(window.0, &raw mut paint) };
        Self { window, paint }
    }

    /// The part of the client area to paint.
    pub(crate) fn region(&self) -> PhysicalRect {
        let rect = self.paint.paint;
        let (left, top) = (rect.left.max(0), rect.top.max(0));
        PhysicalRect::new(
            left as u32,
            top as u32,
            (rect.right - left).max(0) as u32,
            (rect.bottom - top).max(0) as u32,
        )
    }

    /// Paints `region` with the BGRA pixels `bgra`.
    pub(crate) fn draw(&self, region: PhysicalRect, bgra: &[u8]) {
        blit(self.paint.dc, region, bgra);
    }
}

impl Drop for Painter {
    fn drop(&mut self) {
        // SAFETY: ends the paint `begin` started, with its PAINTSTRUCT.
        unsafe { ffi::EndPaint(self.window.0, &raw const self.paint) };
    }
}

/// Paints `region` of `window`'s client area with the BGRA pixels `bgra`,
/// now.
pub(crate) fn draw(window: Hwnd, region: PhysicalRect, bgra: &[u8]) -> Result<(), Error> {
    // SAFETY: a handle-only call; the device context is released below.
    let dc = unsafe { ffi::GetDC(window.0) };
    if dc == 0 {
        return Err(last_error("GetDC"));
    }
    blit(dc, region, bgra);
    // SAFETY: releases the device context GetDC gave for this window.
    unsafe { ffi::ReleaseDC(window.0, dc) };
    Ok(())
}

/// Copies the top-down BGRA image `bgra` of `region`'s size to `region`
/// of the device context `dc`. Images whose length does not match the
/// region are not drawn.
fn blit(dc: Handle, region: PhysicalRect, bgra: &[u8]) {
    let (Ok(width), Ok(height)) = (i32::try_from(region.width), i32::try_from(region.height))
    else {
        return;
    };
    let needed = region.width as usize * region.height as usize * 4;
    if region.is_empty() || bgra.len() != needed {
        return;
    }
    let info = BitmapInfo {
        header: BitmapInfoHeader {
            size: size_of::<BitmapInfoHeader>() as u32,
            width,
            // A negative height makes the image top-down.
            height: -height,
            planes: 1,
            bit_count: 32,
            compression: ffi::BI_RGB,
            ..BitmapInfoHeader::default()
        },
        colors: [0],
    };
    // SAFETY: `bgra` holds the `width * height` 32-bit pixels the header
    // describes, all copied, unscaled.
    unsafe {
        ffi::StretchDIBits(
            dc,
            region.x as i32,
            region.y as i32,
            width,
            height,
            0,
            0,
            width,
            height,
            bgra.as_ptr().cast::<c_void>(),
            &raw const info,
            ffi::DIB_RGB_COLORS,
            ffi::SRCCOPY,
        )
    };
}

/// Shows the system cursor for `icon`.
pub(crate) fn show_cursor(icon: CursorIcon) {
    let resource = usize::from(cursor_resource(icon)) as *const u16;
    // SAFETY: loads a predefined system cursor (MAKEINTRESOURCE), which
    // needs no freeing, and sets it.
    unsafe { ffi::SetCursor(ffi::LoadCursorW(0, resource)) };
}

/// An icon (`HICON`), destroyed when dropped.
#[derive(Debug)]
pub(crate) struct OwnedIcon(Handle);

impl OwnedIcon {
    /// The icon showing `icon`'s pixels.
    pub(crate) fn new(icon: &Icon) -> Result<Self, Error> {
        let size = icon.size();
        let invalid = || Error::InvalidArgument("icon too large for Windows");
        let width = i32::try_from(size.width).map_err(|_| invalid())?;
        let height = i32::try_from(size.height).map_err(|_| invalid())?;
        let color = icon_pixels(icon.rgba());
        let mask = vec![0_u8; mask_len(size.width, size.height)];
        // SAFETY: `color` holds `width * height` 32-bit pixels and `mask`
        // the word-aligned rows of a 1-bit image of that size; the
        // bitmaps are deleted once the icon has copied them.
        unsafe {
            let color = ffi::CreateBitmap(width, height, 1, 32, color.as_ptr().cast());
            let mask = ffi::CreateBitmap(width, height, 1, 1, mask.as_ptr().cast());
            let info = ffi::IconInfo {
                icon: 1,
                x_hotspot: 0,
                y_hotspot: 0,
                mask,
                color,
            };
            let icon = if color == 0 || mask == 0 {
                0
            } else {
                ffi::CreateIconIndirect(&raw const info)
            };
            ffi::DeleteObject(color);
            ffi::DeleteObject(mask);
            if icon == 0 {
                return Err(last_error("CreateIconIndirect"));
            }
            Ok(Self(icon))
        }
    }
}

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        // SAFETY: destroys the icon this value owns.
        unsafe { ffi::DestroyIcon(self.0) };
    }
}

impl Hwnd {
    /// Shows `icon` in the title bar and the task bar (the window keeps
    /// no reference: `icon` must outlive the window or be replaced).
    pub(crate) fn set_icon(self, icon: &OwnedIcon) {
        for kind in [ffi::ICON_SMALL, ffi::ICON_BIG] {
            // SAFETY: a handle-only message; the result is the previous
            // icon, which the window does not own.
            unsafe { ffi::SendMessageW(self.0, ffi::WM_SETICON, kind, icon.0) };
        }
    }
}
