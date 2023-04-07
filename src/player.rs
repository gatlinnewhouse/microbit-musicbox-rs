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

pub enum Event {
    PlayNote,
    NextNote,
    Replay,
    Unknow,
}

pub struct Player<'a, T: timer::Instance, P: pwm::Instance> {
    list: &'a [Melody],
    play_pos: usize,
    note_pos: usize,
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
            play_pos: 0,
            note_pos: 0,
            volume: 100,
            timer,
            buzzer,
        }
    }

    pub fn set_volmue(&mut self, volume: u32) {
        self.volume = cmp::min(100, volume);
    }

    pub fn volume(&self) -> u32 {
        self.volume
    }

    pub fn set_list(&mut self, list: &'a [Melody]) {
        self.timer.stop();
        self.buzzer.stop();
        self.list = list;
    }

    pub fn play(&mut self) {
        if self.list.get(self.play_pos).is_some() {
            self.timer.start();
            self.timer.set_play_duration(1.secs()); // play notes after 1 seconds
        }
    }

    pub fn pause(&mut self) {
        self.timer.stop();
        self.buzzer.stop();
    }

    pub fn next(&mut self) {
        self.timer.stop();
        self.buzzer.stop();
        self.note_pos = 0;
        match self.play_pos.checked_add(1) {
            Some(v) if v < self.list.len() - 1 => {
                self.play_pos = v;
            }
            _ => self.play_pos = 0,
        }
        self.timer.start();
        self.timer.set_play_duration(1.secs());
    }

    pub fn prev(&mut self) {
        self.timer.stop();
        self.buzzer.stop();
        self.note_pos = 0;
        match self.play_pos.checked_sub(1) {
            Some(v) => self.play_pos = v,
            _ => self.play_pos = self.list.len() - 1,
        }
        self.timer.start();
        self.timer.set_play_duration(1.secs());
    }

    pub fn handle_play_event(&mut self) -> Event {
        defmt::debug!("player::tick {}", self.timer.now());
        let mut event = Event::Unknow;
        let play_fired = self.timer.check_play();
        let next_fired = self.timer.check_next();
        if let Some(melody) = self.list.get(self.play_pos) {
            if play_fired {
                let buzzer = &self.buzzer;
                let timer = &self.timer;
                if let Some((tone, delay_ms)) = melody.get(self.note_pos) {
                    buzzer.tone(tone, self.volume);
                    timer.set_play_duration((delay_ms * 1_000).micros());
                    timer.set_next_duration((delay_ms * 900).micros());
                    event = Event::PlayNote;
                } else {
                    self.note_pos = 0;
                    self.timer.stop();
                    self.buzzer.stop();
                    self.timer.start();
                    self.timer.set_play_duration(1.secs());
                    event = Event::Replay;
                }
            } else if next_fired {
                self.note_pos += 1;
                self.buzzer.stop();
                event = Event::NextNote;
            }
        }
        event
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
