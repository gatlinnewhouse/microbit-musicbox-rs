use bsp::hal::gpio::{Input, Pin, PullUp};
use bsp::hal::prelude::*;
use defmt::Format;
use fugit::{ExtU64, TimerDurationU64, TimerInstantU64};

#[derive(Debug, Format, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Click,
    DoubleClick,
    MultiClick(u32),
    LongPressStart,
    LongPressDuring,
    LongPressStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Init = 0,
    Down = 1,
    Up = 2,
    Count = 3,
    Press = 6,
    Pressend = 7,
}

pub struct Button<const TIMER_HZ: u32> {
    pin: Pin<Input<PullUp>>,
    state: State,
    last_state: State,
    cnt_click: u32, // count the number of clicks
    attach_event_fn: Option<fn(Event)>,
    time: TimerInstantU64<TIMER_HZ>,         // record now time
    start_time: TimerInstantU64<TIMER_HZ>,   // start of current input change to checking debouncing
    debounce_ms: TimerDurationU64<TIMER_HZ>, // duration of msecs for debounce times
    click_ms: TimerDurationU64<TIMER_HZ>,    // duration of msecs before a click is detected
    press_ms: TimerDurationU64<TIMER_HZ>, // duration of msecs before a long button press is detected
    hotkey_ms: TimerDurationU64<TIMER_HZ>,   // duration of msecs before a hotkey is detected
}

impl<const TIMER_HZ: u32> Button<TIMER_HZ> {
    pub fn new(pin: Pin<Input<PullUp>>) -> Self {
        Self {
            pin,
            state: State::Init,
            last_state: State::Init,
            cnt_click: 0,
            attach_event_fn: None,
            time: TimerInstantU64::from_ticks(0),
            start_time: TimerInstantU64::from_ticks(0),
            debounce_ms: 50.millis(),
            click_ms: 400.millis(),
            press_ms: 800.millis(),
            hotkey_ms: 200.millis(),
        }
    }

    pub fn attach_event(&mut self, f: fn(Event)) {
        self.attach_event_fn = Some(f);
    }

    pub fn free(self) -> Pin<Input<PullUp>> {
        self.pin
    }

    pub fn reset(&mut self) {
        self.state = State::Init;
        self.last_state = State::Init;
        self.cnt_click = 0;
        self.time = TimerInstantU64::from_ticks(0);
        self.start_time = TimerInstantU64::from_ticks(0);
    }

    pub fn tick(&mut self) {
        self.update_time();

        let active = self.pin.is_low().unwrap();
        let now = self.time;
        let wait_time = now - self.start_time;

        use State::*;
        match self.state {
            Init => {
                if active {
                    self.update_state(Down);
                    self.cnt_click = 0;
                    self.start_time = now;
                }
            }
            Down => {
                if !active && (wait_time < self.debounce_ms) {
                    self.update_state(self.last_state);
                } else if !active {
                    self.update_state(Up);
                } else if active && (wait_time > self.press_ms) {
                    // long pressed start
                    self.update_state(Press);
                    if let Some(f) = self.attach_event_fn {
                        f(Event::LongPressStart)
                    }
                }
            }
            Up => {
                if active && (wait_time < self.debounce_ms) {
                    self.update_state(self.last_state);
                } else if wait_time >= self.debounce_ms {
                    self.cnt_click += 1;
                    self.update_state(Count);
                }
            }
            Count => {
                if active {
                    self.update_state(Down);
                    self.start_time = now;
                } else if wait_time > self.click_ms {
                    if let Some(f) = self.attach_event_fn {
                        match self.cnt_click {
                            1 => f(Event::Click),
                            2 => f(Event::DoubleClick),
                            cnt => f(Event::MultiClick(cnt)),
                        }
                    }
                    self.reset();
                }
            }
            Press => {
                if !active {
                    self.update_state(Pressend);
                    self.start_time = now;
                } else if let Some(f) = self.attach_event_fn {
                    f(Event::LongPressDuring)
                }
            }
            Pressend => {
                if active && (wait_time < self.debounce_ms) {
                    self.update_state(self.last_state);
                } else if wait_time > self.debounce_ms {
                    if let Some(f) = self.attach_event_fn {
                        f(Event::LongPressStop)
                    }
                    self.reset();
                }
            }
        }
    }

    fn update_time(&mut self) {
        self.time += TimerDurationU64::from_ticks(1);
    }

    fn update_state(&mut self, state: State) {
        self.last_state = self.state;
        self.state = state;
    }
}
