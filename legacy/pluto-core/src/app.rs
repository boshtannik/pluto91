//! The app / view runtime, mirroring the pluto firmware's `svc/main` +
//! `common/app` model: an app is a set of views, one is active at a time, and
//! every key press / tick is an [`Event`] delivered to the active view.
//!
//! Everything is generic over the [`Hardware`] type and the concrete app-set
//! enum (generics, not `dyn`), so on the resource-starved MSP430 the whole
//! dispatch monomorphises and inlines like the rest of the firmware.

use crate::hw::Hardware;
use crate::input::KeyId;
use crate::time::DateTime;

/// What can happen on the watch: a tick, a key press, or the auxiliary timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Periodic heartbeat (once per second).
    Tick,
    /// The Up key (Light button) was pressed.
    KeyUp,
    /// The Down key (Mode button) was pressed.
    KeyDown,
    /// The Enter key (Alarm button) was pressed.
    KeyEnter,
    /// Up held past the hold delay (auto-repeats).
    KeyUpLong,
    /// Down held past the hold delay (auto-repeats).
    KeyDownLong,
    /// Enter held past the hold delay.
    KeyEnterLong,
    /// The auxiliary timer reached zero.
    AuxTimer,
}

impl Event {
    pub fn key(key: KeyId) -> Event {
        match key {
            KeyId::Up => Event::KeyUp,
            KeyId::Down => Event::KeyDown,
            KeyId::Enter => Event::KeyEnter,
        }
    }

    pub fn key_long(key: KeyId) -> Event {
        match key {
            KeyId::Up => Event::KeyUpLong,
            KeyId::Down => Event::KeyDownLong,
            KeyId::Enter => Event::KeyEnterLong,
        }
    }
}

/// A view: one screen of an app. Views are plain structs owned by their app;
/// the app's [`App::main`] dispatches to the active view statically. Views
/// receive a handle to the runtime to switch views / launch apps.
pub trait View<H: Hardware, A: App<H>> {
    /// Called when the view becomes active.
    fn enter(&mut self, _time: DateTime, _hw: &mut H) {}
    /// Called when the view stops being active.
    fn leave(&mut self, _time: DateTime, _hw: &mut H) {}
    /// Called on every event while the view is active.
    fn main(&mut self, event: Event, time: DateTime, hw: &mut H, rt: &mut Runtime<H, A>);
}

/// An app: a named program with a set of views, exactly one active at a time.
///
/// Implemented by the concrete app-set enum (one variant per app), so the
/// runtime can hold and switch apps without any dynamic dispatch. Each app
/// owns its view state and dispatches to the active view itself.
pub trait App<H: Hardware>: Clone {
    /// Called when the app becomes current.
    fn enter(&mut self, time: DateTime, hw: &mut H);
    /// Called when the app stops being current.
    fn leave(&mut self, time: DateTime, hw: &mut H);
    /// Called on every event while the app is current.
    fn main(&mut self, event: Event, time: DateTime, hw: &mut H, rt: &mut Runtime<H, Self>);
    /// The currently active view index.
    fn current_view(&self) -> usize;
    /// Switch to `view` within this app (used by the runtime).
    fn set_current_view(&mut self, view: usize);
}

/// A request queued by a view (via [`Runtime`]) and applied after the current
/// event has been processed.
enum Pending<A> {
    Launch(A),
    Exit,
    SetView(usize),
}

/// The app runtime: holds the current app + view and dispatches events.
///
/// `A` is the app-set enum (all apps of this firmware); `home` is the app to
/// return to on [`Runtime::exit`], set on [`Runtime::boot`].
pub struct Runtime<H: Hardware, A: App<H>> {
    app: Option<A>,
    view: usize,
    home: Option<A>,
    pending: Option<Pending<A>>,
    /// Backlight countdown in ticks (mirrors `svc_backlight_process`).
    backlight_ticks: u8,
    /// `H` only appears in the methods.
    _hw: core::marker::PhantomData<fn() -> H>,
}

/// How long the backlight stays on (in ticks) after it was turned on.
const BACKLIGHT_TIMEOUT_TICKS: u8 = 8;

impl<H: Hardware, A: App<H>> Runtime<H, A> {
    pub fn new() -> Self {
        Runtime {
            app: None,
            view: 0,
            home: None,
            pending: None,
            backlight_ticks: 0,
            _hw: core::marker::PhantomData,
        }
    }

    /// The app currently on screen.
    pub fn current_app(&self) -> Option<&A> {
        self.app.as_ref()
    }

    /// The view index currently on screen.
    pub fn current_view(&self) -> usize {
        self.view
    }

    /// Start the watch on `app` (boot). `home` is the app returned to on
    /// [`Runtime::exit`] (the launcher), which may differ from `app`.
    pub fn boot(&mut self, app: A, home: A, time: DateTime, hw: &mut H) {
        self.home = Some(home);
        self.view = 0;
        self.app = Some(app);
        if let Some(a) = self.app.as_mut() {
            a.set_current_view(0);
            a.enter(time, hw);
        }
    }

    /// Switch to a different app.
    pub fn launch(&mut self, app: A) {
        self.pending = Some(Pending::Launch(app));
    }

    /// Go back to the app the watch booted on (the launcher).
    pub fn exit(&mut self) {
        self.pending = Some(Pending::Exit);
    }

    /// Switch to a view within the current app.
    pub fn set_view(&mut self, view: usize) {
        self.pending = Some(Pending::SetView(view));
    }

    /// Process the backlight, deliver the event to the active app, then apply
    /// any queued transition.
    pub fn process(&mut self, event: Event, time: DateTime, hw: &mut H) {
        if event == Event::KeyUpLong {
            self.backlight_ticks = BACKLIGHT_TIMEOUT_TICKS;
            hw.backlight_set(true);
        }
        if self.backlight_ticks > 0 && event != Event::Tick {
            self.backlight_ticks = BACKLIGHT_TIMEOUT_TICKS;
        }
        if event == Event::Tick {
            if self.backlight_ticks > 0 {
                self.backlight_ticks -= 1;
            } else {
                hw.backlight_set(false);
            }
        }
        if let Some(mut app) = self.app.take() {
            app.main(event, time, hw, self);
            self.app = Some(app);
        }
        self.apply_pending(time, hw);
    }

    fn apply_pending(&mut self, time: DateTime, hw: &mut H) {
        while let Some(pending) = self.pending.take() {
            match pending {
                Pending::Launch(app) => {
                    if let Some(mut old) = self.app.take() {
                        old.leave(time, hw);
                    }
                    self.view = 0;
                    self.app = Some(app);
                    if let Some(a) = self.app.as_mut() {
                        a.set_current_view(0);
                        a.enter(time, hw);
                    }
                }
                Pending::Exit => {
                    if let Some(mut old) = self.app.take() {
                        old.leave(time, hw);
                    }
                    self.view = 0;
                    self.app = self.home.clone();
                    if let Some(a) = self.app.as_mut() {
                        a.set_current_view(0);
                        a.enter(time, hw);
                    }
                }
                Pending::SetView(view) => {
                    if let Some(mut app) = self.app.take() {
                        if view != app.current_view() {
                            app.leave(time, hw);
                            app.set_current_view(view);
                            app.enter(time, hw);
                        }
                        self.app = Some(app);
                    }
                }
            }
        }
    }
}
