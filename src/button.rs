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
    INIT = 0,
    DOWN = 1,
    UP = 2,
    COUNT = 3,
    PRESS = 6,
    PRESSEND = 7,
}

pub struct Button<const TIMER_HZ: u32> {
    pin: Pin<Input<PullUp>>,
    state: State,
    last_state: State,
    attach_event_fn: Option<fn(Event)>,

    debounce_ms: TimerDurationU64<TIMER_HZ>, // duration of ticks for debounce times
    click_ms: TimerDurationU64<TIMER_HZ>,    // duration of msecs before a click is detected.
    press_ms: TimerDurationU64<TIMER_HZ>, // duration of msecs before a long button press is detected

    cnt_click: u32,                        // count the number of clicks
    start_time: TimerInstantU64<TIMER_HZ>, // start of current input change to checking debouncing
}

impl<const TIMER_HZ: u32> Button<TIMER_HZ> {
    pub fn new(pin: Pin<Input<PullUp>>) -> Self {
        Self {
            pin,
            state: State::INIT,
            last_state: State::INIT,

            attach_event_fn: None,

            debounce_ms: 50.millis(),
            click_ms: 400.millis(),
            press_ms: 800.millis(),
            cnt_click: 0,
            start_time: TimerInstantU64::from_ticks(0),
        }
    }

    pub fn attach_event(&mut self, f: fn(Event)) {
        self.attach_event_fn = Some(f);
    }

    pub fn free(self) -> Pin<Input<PullUp>> {
        self.pin
    }

    pub fn reset(&mut self) {
        self.state = State::INIT;
        self.last_state = State::INIT;
        self.cnt_click = 0;
        self.start_time = TimerInstantU64::from_ticks(0);
    }

    pub fn tick(&mut self, now: fugit::TimerInstantU64<TIMER_HZ>) {
        let active = self.pin.is_low().unwrap();
        let wait_time = now - self.start_time;

        use State::*;
        match self.state {
            INIT => {
                if active {
                    self.update_state(DOWN);
                    self.cnt_click = 0;
                    self.start_time = now;
                }
            }
            DOWN => {
                if !active && (wait_time < self.debounce_ms) {
                    self.update_state(self.last_state);
                } else if !active {
                    self.update_state(UP);
                } else if active && (wait_time > self.press_ms) {
                    // long pressed start
                    self.update_state(PRESS);
                    self.attach_event_fn.map(|f| f(Event::LongPressStart));
                }
            }
            UP => {
                if active && (wait_time < self.debounce_ms) {
                    self.update_state(self.last_state);
                } else if wait_time >= self.debounce_ms {
                    self.cnt_click += 1;
                    self.update_state(COUNT);
                }
            }
            COUNT => {
                if active {
                    self.update_state(DOWN);
                    self.start_time = now;
                } else if wait_time > self.click_ms {
                    match self.cnt_click {
                        1 => {
                            // single click
                            self.attach_event_fn.map(|f| f(Event::Click));
                        }
                        2 => {
                            // double click
                            self.attach_event_fn.map(|f| f(Event::DoubleClick));
                        }
                        cnt => {
                            // multi click
                            self.attach_event_fn.map(|f| f(Event::MultiClick(cnt)));
                        }
                    }
                    self.reset();
                }
            }
            PRESS => {
                if !active {
                    self.update_state(PRESSEND);
                    self.start_time = now;
                } else {
                    self.attach_event_fn.map(|f| f(Event::LongPressDuring));
                }
            }
            PRESSEND => {
                if active && (wait_time < self.debounce_ms) {
                    self.update_state(self.last_state);
                } else if wait_time > self.debounce_ms {
                    self.attach_event_fn.map(|f| f(Event::LongPressStop));
                    self.reset();
                }
            }
        }
    }

    fn update_state(&mut self, state: State) {
        self.last_state = self.state;
        self.state = state;
    }
}
