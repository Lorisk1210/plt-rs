//! Windows (`NSWindow`), the views pltrs draws in, and cursors.

use std::marker::PhantomData;
use std::ptr::{self, NonNull};

use super::event::EventRef;
use super::foundation::{NsPoint, NsRect, NsSize, string};
use super::runtime::{Id, NO, Object, autoreleasepool, class, send};
use super::{failed, new_object};
use crate::error::Error;
use crate::geometry::LogicalSize;
use crate::macos::convert::{PointRect, cursor_name};
use crate::window::{CursorIcon, WindowId};

/// `NSWindowStyleMaskTitled | Closable | Miniaturizable | Resizable`.
const STYLE: u64 = 1 | 2 | 4 | 8;
/// `NSWindowStyleMaskFullScreen`.
const FULL_SCREEN: u64 = 1 << 14;
/// `NSBackingStoreBuffered`.
const BACKING_BUFFERED: u64 = 2;
/// `NSWindowCollectionBehaviorFullScreenPrimary`: the green button makes
/// the window full screen.
const FULL_SCREEN_PRIMARY: u64 = 1 << 7;
/// `NSTrackingMouseEnteredAndExited | NSTrackingMouseMoved |
/// NSTrackingActiveAlways | NSTrackingInVisibleRect`: report the pointer
/// over the whole view, whether the window is active or not.
const TRACKING: u64 = 0x01 | 0x02 | 0x80 | 0x200;

/// A size in points as an `NSSize`.
const fn ns_size(size: LogicalSize) -> NsSize {
    NsSize {
        width: size.width,
        height: size.height,
    }
}

/// A window, with a title bar, close, minimize and full-screen buttons
/// and resizable borders.
#[derive(Debug)]
pub(crate) struct Window(Id);

impl Window {
    /// A hidden window titled `title` whose content, `view`, is `size`
    /// points large; `view` is also its delegate and first responder.
    pub(crate) fn new(size: LogicalSize, title: &str, view: &View) -> Result<Self, Error> {
        let frame = NsRect {
            origin: NsPoint::default(),
            size: ns_size(size),
        };
        let title = string(title).ok_or_else(|| failed("creating a window title"))?;
        // SAFETY: `new_object` gives an allocated NSWindow, initialized
        // here; then setters taking live objects, numbers and BOOLs. The
        // window retains its content view but not its delegate, which the
        // backend keeps alive as long as the window.
        unsafe {
            let window = new_object(c"NSWindow")?;
            let window: *mut Object = send(
                window,
                c"initWithContentRect:styleMask:backing:defer:",
                (frame, STYLE, BACKING_BUFFERED, NO),
            );
            let window = Id::owned(window).ok_or_else(|| failed("creating a window"))?;
            let pointer = window.as_ptr();
            send::<_, ()>(pointer, c"setReleasedWhenClosed:", (NO,));
            send::<_, ()>(pointer, c"setTitle:", (title.as_ptr(),));
            send::<_, ()>(pointer, c"setCollectionBehavior:", (FULL_SCREEN_PRIMARY,));
            send::<_, ()>(pointer, c"setContentView:", (view.0.as_ptr(),));
            send::<_, ()>(pointer, c"setDelegate:", (view.0.as_ptr(),));
            send::<_, i8>(pointer, c"makeFirstResponder:", (view.0.as_ptr(),));
            Ok(Self(window))
        }
    }

    /// Changes the title.
    pub(crate) fn set_title(&self, title: &str) -> Result<(), Error> {
        let title = string(title).ok_or_else(|| failed("creating a window title"))?;
        // SAFETY: a setter taking a live string, which it copies.
        unsafe { send::<_, ()>(self.0.as_ptr(), c"setTitle:", (title.as_ptr(),)) };
        Ok(())
    }

    /// Asks for a content area of `size` points.
    pub(crate) fn set_content_size(&self, size: LogicalSize) {
        // SAFETY: a setter taking an NSSize.
        unsafe { send::<_, ()>(self.0.as_ptr(), c"setContentSize:", (ns_size(size),)) };
    }

    /// Keeps the content area at least `size` points large.
    pub(crate) fn set_content_min_size(&self, size: LogicalSize) {
        // SAFETY: a setter taking an NSSize.
        unsafe { send::<_, ()>(self.0.as_ptr(), c"setContentMinSize:", (ns_size(size),)) };
    }

    /// Centers the window on its screen.
    pub(crate) fn center(&self) {
        // SAFETY: a method without arguments.
        unsafe { send::<_, ()>(self.0.as_ptr(), c"center", ()) };
    }

    /// Places the window's top-left corner at `point` (screen
    /// coordinates; the origin leaves it in place), and returns where the
    /// next window goes so that windows cascade.
    pub(crate) fn cascade(&self, point: NsPoint) -> NsPoint {
        // SAFETY: a method taking and returning an NSPoint.
        unsafe { send(self.0.as_ptr(), c"cascadeTopLeftFromPoint:", (point,)) }
    }

    /// Shows the window in front, with the keyboard focus.
    pub(crate) fn show(&self) {
        // SAFETY: a method taking a nil sender.
        unsafe {
            send::<_, ()>(
                self.0.as_ptr(),
                c"makeKeyAndOrderFront:",
                (ptr::null_mut::<Object>(),),
            );
        };
    }

    /// Whether the window is full screen.
    pub(crate) fn is_fullscreen(&self) -> bool {
        // SAFETY: a getter returning an NSUInteger.
        unsafe { send::<_, u64>(self.0.as_ptr(), c"styleMask", ()) & FULL_SCREEN != 0 }
    }

    /// Enters or leaves full screen.
    pub(crate) fn toggle_fullscreen(&self) {
        // SAFETY: a method taking a nil sender.
        unsafe {
            send::<_, ()>(
                self.0.as_ptr(),
                c"toggleFullScreen:",
                (ptr::null_mut::<Object>(),),
            );
        };
    }

    /// Makes the window ask `view` for its cursor rectangles again.
    pub(crate) fn invalidate_cursor_rects(&self, view: &View) {
        // SAFETY: a method taking a live view of this window.
        unsafe {
            send::<_, ()>(
                self.0.as_ptr(),
                c"invalidateCursorRectsForView:",
                (view.0.as_ptr(),),
            );
        };
    }

    /// Closes the window, after detaching its delegate.
    pub(crate) fn close(&self) {
        autoreleasepool(|| {
            // SAFETY: a setter taking nil, then a method without
            // arguments; the window is not released when closed.
            unsafe {
                send::<_, ()>(
                    self.0.as_ptr(),
                    c"setDelegate:",
                    (ptr::null_mut::<Object>(),),
                );
                send::<_, ()>(self.0.as_ptr(), c"close", ());
            }
        });
    }
}

/// A view of pltrs's view class, drawing a window's contents.
#[derive(Debug)]
pub(crate) struct View(Id);

impl View {
    /// A view of the class at `class` (pltrs's, see
    /// [`view_class`](super::view_class)), `size` points large, tracking
    /// the pointer over all of it.
    pub(crate) fn new(class: usize, size: LogicalSize) -> Result<Self, Error> {
        let frame = NsRect {
            origin: NsPoint::default(),
            size: ns_size(size),
        };
        let error = || failed("creating a view");
        // SAFETY: an allocated view of a registered NSView subclass,
        // initialized here; then a tracking area owned by the view (which
        // retains it), initialized with live arguments.
        unsafe {
            let view: *mut Object = send(class as *mut Object, c"alloc", ());
            let view = Id::owned(send(view, c"initWithFrame:", (frame,))).ok_or_else(error)?;
            let area = new_object(c"NSTrackingArea")?;
            let area: *mut Object = send(
                area,
                c"initWithRect:options:owner:userInfo:",
                (frame, TRACKING, view.as_ptr(), ptr::null_mut::<Object>()),
            );
            let area = Id::owned(area).ok_or_else(error)?;
            send::<_, ()>(view.as_ptr(), c"addTrackingArea:", (area.as_ptr(),));
            Ok(Self(view))
        }
    }

    /// The view, borrowed.
    pub(crate) fn get(&self) -> ViewRef<'_> {
        ViewRef {
            view: self.0.as_ptr(),
            _life: PhantomData,
        }
    }
}

/// A view of pltrs's view class, borrowed: from a [`View`], or the
/// receiver of one of its methods.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewRef<'a> {
    view: *mut Object,
    _life: PhantomData<&'a Object>,
}

impl ViewRef<'_> {
    /// The view that received a method call.
    ///
    /// # Safety
    ///
    /// `view` is a live view of pltrs's view class, for the lifetime
    /// chosen.
    pub(super) const unsafe fn receiver(view: NonNull<Object>) -> Self {
        Self {
            view: view.as_ptr(),
            _life: PhantomData,
        }
    }

    /// The id of the window the view is the content of: the view's
    /// address.
    pub(crate) fn id(self) -> WindowId {
        WindowId::from_raw(self.view as usize as u64)
    }

    /// The view's bounds, in points.
    pub(crate) fn bounds(self) -> PointRect {
        // SAFETY: a getter returning an NSRect.
        let bounds: NsRect = unsafe { send(self.view, c"bounds", ()) };
        bounds.to_point_rect()
    }

    /// The scale factor of the view's window (1 without a window).
    pub(crate) fn scale_factor(self) -> f64 {
        // SAFETY: the view's window, retained by the application while
        // shown (nil otherwise), and its scale factor.
        unsafe {
            let window: *mut Object = send(self.view, c"window", ());
            if window.is_null() {
                1.0
            } else {
                send(window, c"backingScaleFactor", ())
            }
        }
    }

    /// Where `event` happened, in the view's points (y downwards).
    pub(crate) fn location(self, event: EventRef<'_>) -> (f64, f64) {
        let in_window = event.location_in_window();
        // SAFETY: a conversion from the window's coordinates (a nil view).
        let point: NsPoint = unsafe {
            send(
                self.view,
                c"convertPoint:fromView:",
                (in_window, ptr::null_mut::<Object>()),
            )
        };
        (point.x, point.y)
    }

    /// Marks `rect` (points) to be drawn, and draws it now.
    pub(crate) fn redraw(self, rect: PointRect) {
        // SAFETY: a method taking an NSRect, then one without arguments
        // (which calls `drawRect:`).
        unsafe {
            send::<_, ()>(self.view, c"setNeedsDisplayInRect:", (NsRect::new(rect),));
            send::<_, ()>(self.view, c"displayIfNeeded", ());
        }
    }

    /// Shows `cursor` over `rect` (points); for `resetCursorRects`.
    pub(crate) fn add_cursor_rect(self, rect: PointRect, cursor: CursorIcon) {
        autoreleasepool(|| {
            // SAFETY: a shared system cursor, and a method taking an
            // NSRect and a live cursor.
            unsafe {
                if let Some(cursor) = system_cursor(cursor) {
                    send::<_, ()>(
                        self.view,
                        c"addCursorRect:cursor:",
                        (NsRect::new(rect), cursor),
                    );
                }
            }
        });
    }
}

/// Shows `cursor` now (until the pointer leaves the view).
pub(crate) fn set_cursor(cursor: CursorIcon) {
    autoreleasepool(|| {
        // SAFETY: a shared system cursor, and a method without arguments.
        unsafe {
            if let Some(cursor) = system_cursor(cursor) {
                send::<_, ()>(cursor, c"set", ());
            }
        }
    });
}

/// The shared system cursor for `cursor`.
///
/// # Safety
///
/// An autorelease pool is in place.
unsafe fn system_cursor(cursor: CursorIcon) -> Option<*mut Object> {
    let class = class(c"NSCursor")?;
    // SAFETY: a class method returning a shared cursor.
    let cursor: *mut Object = unsafe { send(class.as_ptr(), cursor_name(cursor), ()) };
    (!cursor.is_null()).then_some(cursor)
}

/// The scale factor of the main screen, before a window shows which
/// screen it is on (1 without a screen).
pub(crate) fn main_screen_scale_factor() -> f64 {
    autoreleasepool(|| {
        // SAFETY: a class method returning the main screen (or nil), and
        // its scale factor.
        unsafe {
            let Some(class) = class(c"NSScreen") else {
                return 1.0;
            };
            let screen: *mut Object = send(class.as_ptr(), c"mainScreen", ());
            if screen.is_null() {
                1.0
            } else {
                send(screen, c"backingScaleFactor", ())
            }
        }
    })
}
