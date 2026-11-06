//! Core Graphics: images of pixels, drawn into the view being drawn.

use std::ffi::c_void;
use std::ptr::NonNull;

use super::foundation::{NsRect, NsSize};
use super::runtime::{Id, Object, autoreleasepool, class, send};
use super::{failed, new_object};
use crate::error::Error;
use crate::geometry::LogicalSize;
use crate::macos::convert::PointRect;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    /// `kCGColorSpaceSRGB`.
    static kCGColorSpaceSRGB: *const c_void;
    fn CGColorSpaceCreateWithName(name: *const c_void) -> *mut c_void;
    fn CGColorSpaceRelease(space: *mut c_void);
    fn CGDataProviderCreateWithData(
        info: *mut c_void,
        data: *const c_void,
        size: usize,
        release: Option<unsafe extern "C" fn(*mut c_void, *const c_void, usize)>,
    ) -> *mut c_void;
    fn CGDataProviderRelease(provider: *mut c_void);
    fn CGImageCreate(
        width: usize,
        height: usize,
        bits_per_component: usize,
        bits_per_pixel: usize,
        bytes_per_row: usize,
        space: *mut c_void,
        bitmap_info: u32,
        provider: *mut c_void,
        decode: *const f64,
        should_interpolate: bool,
        intent: i32,
    ) -> *mut c_void;
    fn CGImageRelease(image: *mut c_void);
    fn CGContextSaveGState(context: *mut c_void);
    fn CGContextRestoreGState(context: *mut c_void);
    fn CGContextTranslateCTM(context: *mut c_void, x: f64, y: f64);
    fn CGContextScaleCTM(context: *mut c_void, x: f64, y: f64);
    fn CGContextSetInterpolationQuality(context: *mut c_void, quality: i32);
    fn CGContextDrawImage(context: *mut c_void, rect: NsRect, image: *mut c_void);
}

/// `kCGBitmapByteOrder32Little | kCGImageAlphaNoneSkipFirst`: opaque
/// pixels stored blue, green, red, unused.
const OPAQUE_BGRX: u32 = (2 << 12) | 6;
/// `kCGBitmapByteOrderDefault | kCGImageAlphaLast`: pixels stored red,
/// green, blue, straight alpha.
const STRAIGHT_RGBA: u32 = 3;
/// `kCGInterpolationNone`.
const INTERPOLATION_NONE: i32 = 1;
/// `kCGRenderingIntentDefault`.
const RENDERING_INTENT_DEFAULT: i32 = 0;

/// A Core Graphics image (`CGImageRef`) owning its pixels, released when
/// dropped.
#[derive(Debug)]
pub(crate) struct Image(NonNull<c_void>);

impl Image {
    /// An opaque sRGB image of `width` × `height` BGRA pixels (alpha
    /// unused).
    pub(crate) fn opaque_bgra(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        Self::new(width, height, pixels, OPAQUE_BGRX)
    }

    /// An sRGB image of `width` × `height` straight-alpha RGBA pixels.
    pub(crate) fn straight_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        Self::new(width, height, pixels, STRAIGHT_RGBA)
    }

    fn new(width: u32, height: u32, pixels: Vec<u8>, layout: u32) -> Option<Self> {
        let (width, height) = (width as usize, height as usize);
        if width == 0 || height == 0 || pixels.len() != width.checked_mul(height)?.checked_mul(4)? {
            return None;
        }
        let len = pixels.len();
        let pixels = Box::new(pixels);
        let data = pixels.as_ptr().cast::<c_void>();
        let info = Box::into_raw(pixels);
        // SAFETY: the provider owns the boxed pixels (whose heap buffer
        // `data` points to) and frees them through `release_pixels`, or
        // they are freed here if there is no provider; the image retains
        // the provider and the colour space, so both are released here.
        unsafe {
            let provider =
                CGDataProviderCreateWithData(info.cast(), data, len, Some(release_pixels));
            if provider.is_null() {
                drop(Box::from_raw(info));
                return None;
            }
            let space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
            let image = CGImageCreate(
                width,
                height,
                8,
                32,
                width * 4,
                space,
                layout,
                provider,
                std::ptr::null(),
                false,
                RENDERING_INTENT_DEFAULT,
            );
            CGColorSpaceRelease(space);
            CGDataProviderRelease(provider);
            NonNull::new(image).map(Self)
        }
    }

    /// Draws the image over `rect` of the view being drawn (in its
    /// flipped coordinates, in points), pixel for pixel.
    pub(crate) fn draw(&self, rect: PointRect) {
        autoreleasepool(|| {
            // SAFETY: the current graphics context, valid in `drawRect:`,
            // and its Core Graphics context, whose state is saved and
            // restored around the drawing.
            unsafe {
                let Some(class) = class(c"NSGraphicsContext") else {
                    return;
                };
                let current: *mut Object = send(class.as_ptr(), c"currentContext", ());
                if current.is_null() {
                    return;
                }
                let context: *mut c_void = send(current, c"CGContext", ());
                if context.is_null() {
                    return;
                }
                CGContextSaveGState(context);
                CGContextSetInterpolationQuality(context, INTERPOLATION_NONE);
                // The view is flipped; images draw upright in unflipped
                // coordinates.
                CGContextTranslateCTM(context, 0.0, rect.y + rect.height);
                CGContextScaleCTM(context, 1.0, -1.0);
                let target = PointRect { y: 0.0, ..rect };
                CGContextDrawImage(context, NsRect::new(target), self.0.as_ptr());
                CGContextRestoreGState(context);
            }
        });
    }
}

/// An `NSImage` of several representations, each image shown at a size
/// in points (a 64-pixel image at 32 points is the 32-point image's 2×
/// representation); the image's own size is the widest of them. `AppKit`
/// draws the representation closest to each size it shows the image at.
pub(crate) fn ns_image(representations: &[(&Image, LogicalSize)]) -> Result<Id, Error> {
    let error = || failed("creating an NSImage");
    let size = |points: LogicalSize| NsSize {
        width: points.width,
        height: points.height,
    };
    let largest = representations
        .iter()
        .map(|&(_, points)| points)
        .max_by(|a, b| a.width.total_cmp(&b.width))
        .ok_or_else(error)?;
    // SAFETY: `new_object` gives allocated objects, initialized here with
    // a size and with live images (which the representations retain);
    // the image retains the representations added to it.
    unsafe {
        let image = Id::owned(send(
            new_object(c"NSImage")?,
            c"initWithSize:",
            (size(largest),),
        ))
        .ok_or_else(error)?;
        for &(cg_image, points) in representations {
            let representation = Id::owned(send(
                new_object(c"NSBitmapImageRep")?,
                c"initWithCGImage:",
                (cg_image.0.as_ptr(),),
            ))
            .ok_or_else(error)?;
            send::<_, ()>(representation.as_ptr(), c"setSize:", (size(points),));
            send::<_, ()>(
                image.as_ptr(),
                c"addRepresentation:",
                (representation.as_ptr(),),
            );
        }
        Ok(image)
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        // SAFETY: releases the image this value owns.
        unsafe { CGImageRelease(self.0.as_ptr()) };
    }
}

/// Frees the pixels of an image, when Core Graphics is done with them.
unsafe extern "C" fn release_pixels(info: *mut c_void, _data: *const c_void, _size: usize) {
    // SAFETY: `info` is the boxed pixel vector `Image::new` gave the
    // provider, which calls this once.
    drop(unsafe { Box::from_raw(info.cast::<Vec<u8>>()) });
}
