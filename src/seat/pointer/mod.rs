//! Utilities to work with pointers and their icons

use std::ops::Deref;

use wayland_client::{
    protocol::{wl_compositor, wl_pointer, wl_seat, wl_shm},
    Attached, DispatchData,
};
mod theme;

pub use self::theme::{ThemeManager, ThemeSpec, ThemedPointer};

/// Wrapper to gracefully handle a missing `libwayland-cursor`
///
/// This wrapper has the same API as `ThemeManager`, but will
/// gracefully handle the case of a missing `libwayland-cursor`
/// by doing nothing.
///
/// It is a convenience wrapper to handle systems where
/// `libwayland-client.so` is available but not `libwayland-cursor.so`.
pub enum AutoThemer {
    /// The theme could be loaded
    Themed(ThemeManager),
    /// `libwayland-cursor.so` is not available
    UnThemed,
}

impl AutoThemer {
    /// Load a system pointer theme
    ///
    /// Will use the default theme of the system if name is `None`.
    ///
    /// Falls back to `UnThemed` if `libwayland-cursor` is not available.
    pub fn init(
        theme: ThemeSpec,
        compositor: Attached<wl_compositor::WlCompositor>,
        shm: Attached<wl_shm::WlShm>,
    ) -> AutoThemer {
        match ThemeManager::init(theme, compositor, shm) {
            Ok(mgr) => AutoThemer::Themed(mgr),
            Err(()) => AutoThemer::UnThemed,
        }
    }

    /// Wrap a pointer to theme it
    pub fn theme_pointer(&self, pointer: wl_pointer::WlPointer) -> AutoPointer {
        match *self {
            AutoThemer::Themed(ref mgr) => AutoPointer::Themed(mgr.theme_pointer(pointer)),
            AutoThemer::UnThemed => AutoPointer::UnThemed(pointer),
        }
    }

    /// Initialize a new pointer as a ThemedPointer with an adapter implementation
    ///
    /// You need to provide an implementation as if implementing a `wl_pointer`, but
    /// it will receive as `meta` argument an `AutoPointer` wrapping your pointer,
    /// rather than a `WlPointer`.
    pub fn theme_pointer_with_impl<Impl>(
        &self,
        seat: &Attached<wl_seat::WlSeat>,
        mut callback: Impl,
    ) -> AutoPointer
    where
        Impl: FnMut(wl_pointer::Event, AutoPointer, DispatchData) + 'static,
    {
        match *self {
            AutoThemer::Themed(ref mgr) => {
                let pointer = mgr.theme_pointer_with_impl(seat, move |event, seat, ddata| {
                    callback(event, AutoPointer::Themed(seat), ddata)
                });
                AutoPointer::Themed(pointer)
            }
            AutoThemer::UnThemed => {
                let pointer = seat.get_pointer();
                pointer.quick_assign(move |ptr, event, data| {
                    callback(event, AutoPointer::UnThemed((*ptr).clone().detach()), data)
                });
                AutoPointer::UnThemed((*pointer).clone().detach())
            }
        }
    }
}

/// A pointer wrapper to gracefully handle a missing `libwayland-cursor`
///
/// It has the same API as `ThemedPointer`, but falls back to doing nothing
/// in its `Unthemed` variant.
pub enum AutoPointer {
    /// The `ThemedPointer`
    Themed(ThemedPointer),
    /// The regular pointer if theme capability is not available
    UnThemed(wl_pointer::WlPointer),
}

impl AutoPointer {
    /// Change the cursor to the given cursor name
    ///
    /// Possible names depend on the theme. Does nothing and returns
    /// `Err(())` if given name is not available.
    ///
    /// Does nothing an returns `Ok(())` if no theme is loaded (if
    /// `wayland-cursor` is not available).
    ///
    /// If this is done as an answer to an input event, you need to provide
    /// the associated serial otherwise the server may ignore the request.
    pub fn set_cursor(&self, name: &str, serial: Option<u32>) -> Result<(), ()> {
        match *self {
            AutoPointer::Themed(ref themed) => themed.set_cursor(name, serial),
            AutoPointer::UnThemed(_) => Ok(()),
        }
    }
}

impl Deref for AutoPointer {
    type Target = wl_pointer::WlPointer;
    fn deref(&self) -> &wl_pointer::WlPointer {
        match *self {
            AutoPointer::Themed(ref themed) => &**themed,
            AutoPointer::UnThemed(ref ptr) => ptr,
        }
    }
}

impl Clone for AutoPointer {
    fn clone(&self) -> AutoPointer {
        match *self {
            AutoPointer::Themed(ref themed) => AutoPointer::Themed(themed.clone()),
            AutoPointer::UnThemed(ref ptr) => AutoPointer::UnThemed(ptr.clone()),
        }
    }
}
