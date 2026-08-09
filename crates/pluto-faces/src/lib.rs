//! The faces (programs) of the watch.
//!
//! Each face is a struct implementing [`Face`]. [`Faces`] is the enum that
//! links them all together and is the type parameter of the runtime. To add a
//! new face: write a module below, add it as a variant of [`Faces`], delegate
//! the trait methods, and add it to [`FaceSet::all`].
#![no_std]

mod alarm;
mod simple_clock;

use pluto_core::face::{AlarmAction, ChordEvent, Face, FaceContext, GestureEvent};
use pluto_core::watch::FaceSet;
use pluto_core::Hardware;

pub use alarm::{Alarm, AlarmDay, AlarmField, AlarmMode};
pub use simple_clock::SimpleClock;

/// All faces, linked together as an enum.
#[derive(Clone, Copy, PartialEq)]
pub enum Faces {
    SimpleClock(SimpleClock),
    Alarm(Alarm),
}

impl Default for Faces {
    fn default() -> Self {
        Faces::SimpleClock(SimpleClock::new())
    }
}

impl Face for Faces {
    fn init(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        match self {
            Faces::SimpleClock(f) => f.init(ctx, hw),
            Faces::Alarm(f) => f.init(ctx, hw),
        }
    }

    fn tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        match self {
            Faces::SimpleClock(f) => f.tick(ctx, hw),
            Faces::Alarm(f) => f.tick(ctx, hw),
        }
    }

    fn background_tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        match self {
            Faces::SimpleClock(f) => f.background_tick(ctx, hw),
            Faces::Alarm(f) => f.background_tick(ctx, hw),
        }
    }

    fn button(&mut self, event: GestureEvent, ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        match self {
            Faces::SimpleClock(f) => f.button(event, ctx, hw),
            Faces::Alarm(f) => f.button(event, ctx, hw),
        }
    }

    fn chord(&mut self, event: ChordEvent, ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        match self {
            Faces::SimpleClock(f) => f.chord(event, ctx, hw),
            Faces::Alarm(f) => f.chord(event, ctx, hw),
        }
    }

    fn alarm_action(&self) -> AlarmAction {
        match self {
            Faces::SimpleClock(f) => f.alarm_action(),
            Faces::Alarm(f) => f.alarm_action(),
        }
    }
}

static ALL_FACES: [Faces; 2] = [
    Faces::SimpleClock(SimpleClock::new()),
    Faces::Alarm(Alarm::new()),
];

impl FaceSet for Faces {
    const LEN: usize = 2;
    type Faces = [Faces; 2];

    fn all() -> &'static [Self] {
        &ALL_FACES
    }

    fn faces() -> Self::Faces {
        ALL_FACES
    }
}
