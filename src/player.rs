use core::cmp;

use bsp::hal::{
    gpio::{Output, Pin, PushPull},
    pwm, timer,
};
use fugit::ExtU32;

use self::inner::{PlayerBuzzer, PlayerTimer};
use crate::{melody::Melody, tone::Tone};

type Instant = fugit::Instant<u32, 1, 1_000_000>;
type Duration = fugit::Duration<u32, 1, 1_000_000>;

const DEFAULT_PLAY_DURATION: Duration = Duration::from_ticks(1 * 1000 * 1000);

enum State {
    Play { pos: usize, progress: usize },
    Pause { pos: usize, progress: usize },
    Stop,
}

pub struct Player<'a, T: timer::Instance, P: pwm::Instance> {
    list: &'a [Melody],
    state: State,
    volume: u32,
    timer: PlayerTimer<T>,
    buzzer: PlayerBuzzer<P>,
}

impl<'a, T: timer::Instance, P: pwm::Instance> Player<'a, T, P> {
    pub fn new(timer: T, pwm: P, pin: Pin<Output<PushPull>>, list: &'a [Melody]) -> Self {
        let timer = PlayerTimer::new(timer);
        let buzzer = PlayerBuzzer::new(pwm, pin);
        Self {
            list,
            state: State::Stop,
            volume: 100,
            timer,
            buzzer,
        }
    }

    pub fn volume_add(&mut self, volume: u32) {
        self.volume = self.volume.saturating_add(volume).min(100);
    }

    pub fn volume_sub(&mut self, volume: u32) {
        self.volume = self.volume.saturating_sub(volume);
    }

    pub fn volume(&self) -> u32 {
        self.volume
    }

    pub fn set_list(&mut self, list: &'a [Melody]) {
        self.stop();
        self.list = list;
    }

    pub fn stop(&mut self) {
        self.timer.stop();
        self.buzzer.stop();
        self.state = State::Stop;
    }

    pub fn play(&mut self) {
        if let Some(next_state) = match self.state {
            State::Stop => Some(State::Play {
                pos: 0,
                progress: 0,
            }),
            State::Pause { pos, progress } => Some(State::Play { pos, progress }),
            _ => None,
        } {
            self.state = next_state;
            self.timer.start();
            self.timer.set_play_duration(DEFAULT_PLAY_DURATION);
        }
    }

    pub fn pause(&mut self) {
        if let Some(next_state) = match self.state {
            State::Play { pos, progress } => Some(State::Pause { pos, progress }),
            _ => None,
        } {
            self.timer.stop();
            self.buzzer.stop();
            self.state = next_state;
        }
    }

    pub fn next(&mut self) {
        let next_pos = self.next_pos();
        self._start_play(next_pos);
    }

    pub fn prev(&mut self) {
        let prev_pos = self.prev_pos();
        self._start_play(prev_pos);
    }

    pub fn handle_play_event(&mut self) {
        defmt::debug!("player::tick {}", self.timer.now());
        if let State::Play { pos, progress } = self.state {
            let play_fired = self.timer.check_play();
            let next_fired = self.timer.check_next();

            if let Some(melody) = self.list.get(pos) {
                if play_fired {
                    let buzzer = &self.buzzer;
                    let timer = &self.timer;
                    if let Some((tone, delay_ms)) = melody.get(progress) {
                        // play that note for 90% duration, leaving 10% pause
                        buzzer.tone(tone, self.volume);
                        timer.set_play_duration((delay_ms * 1_000).micros());
                        timer.set_next_duration((delay_ms * 900).micros());
                    } else {
                        self._start_play(pos);
                    }
                } else if next_fired {
                    self.state = State::Play {
                        pos,
                        progress: progress + 1,
                    };
                    self.buzzer.stop();
                }
            }
        }
    }

    fn prev_pos(&self) -> usize {
        let max_pos = self.list.len() - 1;
        let pos = match self.state {
            State::Play { pos, .. } => pos,
            State::Pause { pos, .. } => pos,
            State::Stop => 0,
        };

        if pos == 0 {
            max_pos
        } else {
            pos - 1
        }
    }

    fn next_pos(&self) -> usize {
        let max_pos = self.list.len() - 1;
        let pos = match self.state {
            State::Play { pos, .. } => pos,
            State::Pause { pos, .. } => pos,
            State::Stop => 0,
        };

        if pos == max_pos {
            0
        } else {
            pos + 1
        }
    }

    fn _start_play(&mut self, pos: usize) {
        self.stop();
        self.state = State::Play {
            pos: pos,
            progress: 0,
        };
        self.timer.start();
        self.timer.set_play_duration(DEFAULT_PLAY_DURATION);
    }
}

mod inner {
    use super::*;

    pub(super) struct PlayerBuzzer<T: pwm::Instance>(pwm::Pwm<T>);

    impl<T: pwm::Instance> PlayerBuzzer<T> {
        pub fn new(pwm: T, pin: Pin<Output<PushPull>>) -> Self {
            let buzzer = pwm::Pwm::new(pwm);
            buzzer
                .set_counter_mode(pwm::CounterMode::UpAndDown)
                .set_output_pin(pwm::Channel::C0, pin)
                .disable();
            Self(buzzer)
        }

        pub fn tone(&self, tone: Tone, volume: u32) {
            self.0.disable();
            if tone != Tone::REST {
                self.0.set_period(tone.hz());
                self.update_volume(volume);
                self.0.enable();
            }
        }

        pub fn stop(&self) {
            self.0.disable();
        }

        #[inline(always)]
        fn update_volume(&self, volume: u32) {
            let max_duty = self.0.max_duty() as f32;
            let min_vol = max_duty * 0.2;
            let max_vol = max_duty * 0.5;
            let vol = (max_vol - min_vol) * (volume as f32 / 100_f32);
            self.0.set_duty_on(pwm::Channel::C0, (min_vol + vol) as u16);
        }
    }

    pub(super) struct PlayerTimer<T: timer::Instance>(T);

    impl<T: timer::Instance> PlayerTimer<T> {
        pub fn new(timer: T) -> Self {
            let timer0 = timer.as_timer0();
            timer0.tasks_stop.write(|w| w.tasks_stop().set_bit());
            timer0.tasks_clear.write(|w| w.tasks_clear().set_bit());
            timer0.bitmode.write(|w| w.bitmode()._32bit());
            timer0.prescaler.write(|w| unsafe { w.prescaler().bits(4) }); // 1 Mhz
            timer0.intenset.write(|w| w.compare1().set_bit());
            timer0.intenset.write(|w| w.compare2().set_bit());
            Self(timer)
        }

        pub fn start(&self) {
            let timer = self.0.as_timer0();
            timer.tasks_start.write(|w| unsafe { w.bits(1) });
        }

        pub fn stop(&self) {
            let timer = self.0.as_timer0();
            timer.tasks_stop.write(|w| unsafe { w.bits(1) });
            timer.tasks_clear.write(|w| unsafe { w.bits(1) });
        }

        pub fn set_play_duration(&self, duration: Duration) {
            self.set_duration_for_cc(1, duration)
        }

        pub fn set_next_duration(&self, duration: Duration) {
            self.set_duration_for_cc(2, duration)
        }

        pub fn check_play(&self) -> bool {
            self.check_fired_for_cc(1)
        }

        pub fn check_next(&self) -> bool {
            self.check_fired_for_cc(2)
        }

        #[inline(always)]
        pub fn now(&self) -> Instant {
            let timer = self.0.as_timer0();
            timer.tasks_capture[0].write(|w| unsafe { w.bits(1) });
            Instant::from_ticks(timer.cc[0].read().bits())
        }

        #[inline(always)]
        fn set_duration_for_cc(&self, pos: usize, duration: Duration) {
            let timer = self.0.as_timer0();
            let now = self.now();
            let instant = now + duration;
            timer.cc[pos].write(|w| unsafe { w.cc().bits(instant.duration_since_epoch().ticks()) });
        }

        #[inline(always)]
        fn check_fired_for_cc(&self, pos: usize) -> bool {
            let timer = self.0.as_timer0();
            let reg = &timer.events_compare[pos];
            let fired = reg.read().bits() != 0;
            if fired {
                reg.reset();
            }
            fired
        }
    }
}
