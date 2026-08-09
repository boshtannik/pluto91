//! Pluto apps: the concrete app set for the watch.
//!
//! The launcher, the time clock face and its configuration menu, and the
//! configuration app. The [`AppSet`] enum implements the [`App`] trait, so
//! the [`Runtime`] holds and switches apps statically (no `dyn`).

#![no_std]

pub mod conf;
pub mod launcher;
pub mod menu;
pub mod settings;
pub mod time;

use pluto_core::{App, DateTime, Event, Hardware, Runtime};

use conf::ConfApp;
use launcher::Launcher;
use time::TimeApp;

/// All apps of this firmware as an enum; one variant per app. The [`App`]
/// impl just forwards to the active variant, so the whole dispatch
/// monomorphises.
#[derive(Clone)]
pub enum AppSet {
    Launcher(Launcher),
    Time(TimeApp),
    Conf(ConfApp),
}

impl<H: Hardware> App<H> for AppSet {
    fn enter(&mut self, time: DateTime, hw: &mut H) {
        match self {
            AppSet::Launcher(a) => a.enter(time, hw),
            AppSet::Time(a) => a.enter(time, hw),
            AppSet::Conf(a) => a.enter(time, hw),
        }
    }

    fn leave(&mut self, time: DateTime, hw: &mut H) {
        match self {
            AppSet::Launcher(a) => a.leave(time, hw),
            AppSet::Time(a) => a.leave(time, hw),
            AppSet::Conf(a) => a.leave(time, hw),
        }
    }

    fn main(&mut self, event: Event, time: DateTime, hw: &mut H, rt: &mut Runtime<H, AppSet>) {
        match self {
            AppSet::Launcher(a) => a.main(event, time, hw, rt),
            AppSet::Time(a) => a.main(event, time, hw, rt),
            AppSet::Conf(a) => a.main(event, time, hw, rt),
        }
    }

    fn current_view(&self) -> usize {
        match self {
            AppSet::Launcher(a) => a.current_view(),
            AppSet::Time(a) => a.current_view(),
            AppSet::Conf(a) => a.current_view(),
        }
    }

    fn set_current_view(&mut self, view: usize) {
        match self {
            AppSet::Launcher(a) => a.set_current_view(view),
            AppSet::Time(a) => a.set_current_view(view),
            AppSet::Conf(a) => a.set_current_view(view),
        }
    }
}
