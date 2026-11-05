//! The Objective-C runtime: objects and their reference counts,
//! selectors, message sends, autorelease pools and class definitions.

use std::ffi::{CStr, CString, c_char, c_void};
use std::marker::PhantomData;
use std::ptr::NonNull;

/// An Objective-C object, only ever handled through pointers.
#[repr(C)]
pub(crate) struct Object {
    _opaque: [u8; 0],
    /// Objects are not thread-safe in general.
    _not_send: PhantomData<*mut u8>,
}

/// A selector (`SEL`).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sel(*const c_void);

impl Sel {
    /// No selector (a nil `SEL`).
    pub(crate) const NONE: Self = Self(std::ptr::null());
}

#[link(name = "objc", kind = "dylib")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut Object;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend();
    #[cfg(target_arch = "x86_64")]
    fn objc_msgSend_stret();
    fn objc_allocateClassPair(
        superclass: *mut Object,
        name: *const c_char,
        extra_bytes: usize,
    ) -> *mut Object;
    fn objc_registerClassPair(class: *mut Object);
    fn class_addMethod(
        class: *mut Object,
        name: Sel,
        implementation: *const c_void,
        types: *const c_char,
    ) -> i8;
    fn objc_release(object: *mut Object);
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}

/// `YES`, as `BOOL` is passed (a signed char on x86-64, a one-byte bool
/// on arm64: the same in registers).
pub(crate) const YES: i8 = 1;
/// `NO`.
pub(crate) const NO: i8 = 0;

/// The selector named `name`.
pub(crate) fn sel(name: &CStr) -> Sel {
    // SAFETY: the name is NUL-terminated; registering a selector name is
    // idempotent and thread-safe.
    unsafe { sel_registerName(name.as_ptr()) }
}

/// The class named `name`, if it exists.
pub(crate) fn class(name: &CStr) -> Option<NonNull<Object>> {
    // SAFETY: the name is NUL-terminated; looking up a class has no side
    // effects.
    NonNull::new(unsafe { objc_getClass(name.as_ptr()) })
}

/// The arguments of a message: tuples of the C types the method takes.
pub(crate) trait Arguments: Copy {
    /// Sends `selector` with these arguments to `receiver`.
    ///
    /// # Safety
    ///
    /// `receiver` is a live object (or nil), whose method for `selector`
    /// takes exactly these argument types and returns `R`.
    unsafe fn send<R>(self, receiver: *mut Object, selector: Sel) -> R;
}

/// The entry point that sends messages returning `R`: on x86-64,
/// structures larger than 16 bytes are returned through memory, by
/// `objc_msgSend_stret`.
fn entry_point<R>() -> *const c_void {
    if cfg!(target_arch = "x86_64") && size_of::<R>() > 16 {
        struct_return_entry_point()
    } else {
        objc_msgSend as *const c_void
    }
}

/// `objc_msgSend_stret`, for structures returned through memory.
#[cfg(target_arch = "x86_64")]
fn struct_return_entry_point() -> *const c_void {
    objc_msgSend_stret as *const c_void
}

/// arm64 returns every structure through `objc_msgSend`.
#[cfg(not(target_arch = "x86_64"))]
fn struct_return_entry_point() -> *const c_void {
    objc_msgSend as *const c_void
}

macro_rules! arguments {
    ($($name:ident: $ty:ident),*) => {
        impl<$($ty: Copy),*> Arguments for ($($ty,)*) {
            unsafe fn send<R>(self, receiver: *mut Object, selector: Sel) -> R {
                let ($($name,)*) = self;
                // SAFETY: the entry point is the runtime's message send for
                // this return type, called as the method's C function type,
                // which the caller vouches for.
                unsafe {
                    let send: unsafe extern "C" fn(*mut Object, Sel $(, $ty)*) -> R =
                        std::mem::transmute(entry_point::<R>());
                    send(receiver, selector $(, $name)*)
                }
            }
        }
    };
}

arguments!();
arguments!(a: A);
arguments!(a: A, b: B);
arguments!(a: A, b: B, c: C);
arguments!(a: A, b: B, c: C, d: D);
arguments!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I);
arguments!(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J);

/// Sends `selector` with `arguments` to `receiver`, returning `R`.
///
/// # Safety
///
/// As [`Arguments::send`]: `receiver` is live or nil, and its method for
/// `selector` has exactly these argument and return types.
pub(crate) unsafe fn send<A: Arguments, R>(
    receiver: *mut Object,
    selector: &CStr,
    arguments: A,
) -> R {
    // SAFETY: the caller's guarantee.
    unsafe { arguments.send(receiver, sel(selector)) }
}

/// A strong reference to an object, released when dropped.
#[derive(Debug)]
pub(crate) struct Id(NonNull<Object>);

impl Id {
    /// Takes over a reference the caller owns (from `alloc`/`init`,
    /// `new`, `copy`); `None` for nil.
    ///
    /// # Safety
    ///
    /// `object` is nil or a live object whose reference is the caller's
    /// to give up.
    pub(crate) unsafe fn owned(object: *mut Object) -> Option<Self> {
        NonNull::new(object).map(Self)
    }

    /// The object.
    pub(crate) const fn as_ptr(&self) -> *mut Object {
        self.0.as_ptr()
    }
}

impl Drop for Id {
    fn drop(&mut self) {
        // SAFETY: gives up the reference this value owns.
        unsafe { objc_release(self.0.as_ptr()) };
    }
}

/// Runs `f` with an autorelease pool, which releases the objects
/// autoreleased meanwhile when `f` returns. Nothing autoreleased may
/// outlive `f`: the wrappers using it return Rust values or [`Id`]s.
pub(crate) fn autoreleasepool<T>(f: impl FnOnce() -> T) -> T {
    /// Pops the pool, even when `f` panics.
    struct Pool(*mut c_void);
    impl Drop for Pool {
        fn drop(&mut self) {
            // SAFETY: pops the pool pushed below, the innermost one.
            unsafe { objc_autoreleasePoolPop(self.0) };
        }
    }
    // SAFETY: pushes a pool, popped by `Pool`'s drop in order.
    let _pool = Pool(unsafe { objc_autoreleasePoolPush() });
    f()
}

/// A method to add to a class: its selector, its implementation and
/// its type encoding.
pub(crate) struct Method {
    /// The selector.
    pub(crate) selector: &'static CStr,
    /// The implementation, an `extern "C"` function taking the receiver,
    /// the selector and the method's arguments.
    pub(crate) implementation: *const c_void,
    /// The Objective-C type encoding of the method.
    pub(crate) types: &'static CStr,
}

/// Defines and registers the class `name`, a subclass of `superclass`
/// with `methods`; returns its address.
///
/// # Safety
///
/// Each method's implementation is an `extern "C"` function whose
/// parameters (receiver, selector, arguments) and result match `types`
/// and the method `AppKit` declares for that selector.
pub(crate) unsafe fn define_class(
    name: &str,
    superclass: &CStr,
    methods: &[Method],
) -> Option<usize> {
    let name = CString::new(name).ok()?;
    let superclass = class(superclass)?;
    // SAFETY: a new class pair named `name` (nil if the name is taken),
    // given its methods (whose types the caller vouches for), then
    // registered once.
    unsafe {
        let class = objc_allocateClassPair(superclass.as_ptr(), name.as_ptr(), 0);
        if class.is_null() {
            return None;
        }
        for method in methods {
            let selector = sel(method.selector);
            class_addMethod(
                class,
                selector,
                method.implementation,
                method.types.as_ptr(),
            );
        }
        objc_registerClassPair(class);
        Some(class as usize)
    }
}
