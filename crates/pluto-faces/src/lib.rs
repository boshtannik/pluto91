//! The faces (programs) of the watch.
//!
//! Each face is a struct implementing [`Face`]. [`Faces`] is the enum that
//! links them all together and is the type parameter of the runtime. To add a
//! new face: write a module below, add it as a variant of [`Faces`], delegate
//! the trait methods, and add it to [`FaceSet::all`].
//!
//! Which faces are actually compiled is decided by `faces.toml` in the crate
//! root: the `build.rs` script turns every listed face into a `face_*` cfg
//! flag, and the modules and enum variants below are gated on those flags.
//! Faces not listed in the file are dropped from the binary.
#![no_std]

#[cfg(face_simple_clock)]
mod simple_clock;
#[cfg(face_alarm)]
mod alarm;
#[cfg(face_simple_alarm)]
mod simple_alarm;
#[cfg(face_timer)]
mod timer;
#[cfg(face_calendar)]
mod calendar;

use pluto_core::face::{AlarmAction, ChordEvent, Face, FaceContext, GestureEvent};
use pluto_core::watch::FaceSet;
use pluto_core::Hardware;

#[cfg(face_simple_clock)]
pub use simple_clock::SimpleClock;
#[cfg(face_alarm)]
pub use alarm::{Alarm, AlarmDay, AlarmField, AlarmMode};
#[cfg(face_simple_alarm)]
pub use simple_alarm::{SimpleAlarm, SimpleAlarmField, SimpleAlarmMode};
#[cfg(face_timer)]
pub use timer::{Timer, TimerField, TimerMode};
#[cfg(face_calendar)]
pub use calendar::{Calendar, CalendarField, CalendarMode};

/// Number of enabled faces (the sum of one-per-face cfg counters).
const FACE_COUNT: usize = if cfg!(face_simple_clock) { 1 } else { 0 }
    + if cfg!(face_alarm) { 1 } else { 0 }
    + if cfg!(face_simple_alarm) { 1 } else { 0 }
    + if cfg!(face_timer) { 1 } else { 0 }
    + if cfg!(face_calendar) { 1 } else { 0 };

/// All faces, linked together as an enum. `simple_clock` is always the first
/// variant: the runtime boots into it and it is the `Default`.
#[derive(Clone, Copy, PartialEq)]
pub enum Faces {
    #[cfg(face_simple_clock)]
    SimpleClock(SimpleClock),
    #[cfg(face_alarm)]
    Alarm(Alarm),
    #[cfg(face_simple_alarm)]
    SimpleAlarm(SimpleAlarm),
    #[cfg(face_timer)]
    Timer(Timer),
    #[cfg(face_calendar)]
    Calendar(Calendar),
}

impl Default for Faces {
    fn default() -> Self {
        Faces::SimpleClock(SimpleClock::new())
    }
}

impl Face for Faces {
    fn init(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        match self {
            #[cfg(face_simple_clock)]
            Faces::SimpleClock(f) => f.init(ctx, hw),
            #[cfg(face_alarm)]
            Faces::Alarm(f) => f.init(ctx, hw),
            #[cfg(face_simple_alarm)]
            Faces::SimpleAlarm(f) => f.init(ctx, hw),
            #[cfg(face_timer)]
            Faces::Timer(f) => f.init(ctx, hw),
            #[cfg(face_calendar)]
            Faces::Calendar(f) => f.init(ctx, hw),
        }
    }

    fn tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) {
        match self {
            #[cfg(face_simple_clock)]
            Faces::SimpleClock(f) => f.tick(ctx, hw),
            #[cfg(face_alarm)]
            Faces::Alarm(f) => f.tick(ctx, hw),
            #[cfg(face_simple_alarm)]
            Faces::SimpleAlarm(f) => f.tick(ctx, hw),
            #[cfg(face_timer)]
            Faces::Timer(f) => f.tick(ctx, hw),
            #[cfg(face_calendar)]
            Faces::Calendar(f) => f.tick(ctx, hw),
        }
    }

    fn background_tick(&mut self, ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        match self {
            #[cfg(face_simple_clock)]
            Faces::SimpleClock(f) => f.background_tick(ctx, hw),
            #[cfg(face_alarm)]
            Faces::Alarm(f) => f.background_tick(ctx, hw),
            #[cfg(face_simple_alarm)]
            Faces::SimpleAlarm(f) => f.background_tick(ctx, hw),
            #[cfg(face_timer)]
            Faces::Timer(f) => f.background_tick(ctx, hw),
            #[cfg(face_calendar)]
            Faces::Calendar(f) => f.background_tick(ctx, hw),
        }
    }

    fn button(&mut self, event: GestureEvent, ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        match self {
            #[cfg(face_simple_clock)]
            Faces::SimpleClock(f) => f.button(event, ctx, hw),
            #[cfg(face_alarm)]
            Faces::Alarm(f) => f.button(event, ctx, hw),
            #[cfg(face_simple_alarm)]
            Faces::SimpleAlarm(f) => f.button(event, ctx, hw),
            #[cfg(face_timer)]
            Faces::Timer(f) => f.button(event, ctx, hw),
            #[cfg(face_calendar)]
            Faces::Calendar(f) => f.button(event, ctx, hw),
        }
    }

    fn chord(&mut self, event: ChordEvent, ctx: &FaceContext, hw: &mut impl Hardware) -> bool {
        match self {
            #[cfg(face_simple_clock)]
            Faces::SimpleClock(f) => f.chord(event, ctx, hw),
            #[cfg(face_alarm)]
            Faces::Alarm(f) => f.chord(event, ctx, hw),
            #[cfg(face_simple_alarm)]
            Faces::SimpleAlarm(f) => f.chord(event, ctx, hw),
            #[cfg(face_timer)]
            Faces::Timer(f) => f.chord(event, ctx, hw),
            #[cfg(face_calendar)]
            Faces::Calendar(f) => f.chord(event, ctx, hw),
        }
    }

    fn alarm_action(&self) -> AlarmAction {
        match self {
            #[cfg(face_simple_clock)]
            Faces::SimpleClock(f) => f.alarm_action(),
            #[cfg(face_alarm)]
            Faces::Alarm(f) => f.alarm_action(),
            #[cfg(face_simple_alarm)]
            Faces::SimpleAlarm(f) => f.alarm_action(),
            #[cfg(face_timer)]
            Faces::Timer(f) => f.alarm_action(),
            #[cfg(face_calendar)]
            Faces::Calendar(f) => f.alarm_action(),
        }
    }
}

/// One instance of every enabled face, in the enum order above (the Mode
/// cycle order).
#[allow(unused_assignments)] // the trailing `i += 1` is unused when faces are disabled
static ALL_FACES: [Faces; FACE_COUNT] = {
    let mut faces = [Faces::SimpleClock(SimpleClock::new()); FACE_COUNT];
    let mut i = 0;
    #[cfg(face_simple_clock)]
    {
        faces[i] = Faces::SimpleClock(SimpleClock::new());
        i += 1;
    }
    #[cfg(face_alarm)]
    {
        faces[i] = Faces::Alarm(Alarm::new());
        i += 1;
    }
    #[cfg(face_simple_alarm)]
    {
        faces[i] = Faces::SimpleAlarm(SimpleAlarm::new());
        i += 1;
    }
    #[cfg(face_timer)]
    {
        faces[i] = Faces::Timer(Timer::new());
        i += 1;
    }
    #[cfg(face_calendar)]
    {
        faces[i] = Faces::Calendar(Calendar::new());
        i += 1;
    }
    faces
};

impl FaceSet for Faces {
    const LEN: usize = FACE_COUNT;
    type Faces = [Faces; FACE_COUNT];

    fn all() -> &'static [Self] {
        &ALL_FACES
    }

    fn faces() -> Self::Faces {
        ALL_FACES
    }
}
