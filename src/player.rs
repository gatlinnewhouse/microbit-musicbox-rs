use core::cmp::{max, min};

use bsp::hal::{
    gpio::{Output, Pin, PushPull},
    pwm, timer,
};
use fugit::ExtU32;

use self::inner::{PlayerBuzzer, PlayerTimer};
use crate::{melody::Melody, tone::Tone};

type Instant = fugit::Instant<u32, 1, 1_000_000>;
type Duration = fugit::Duration<u32, 1, 1_000_000>;

pub struct Player<'a, T: timer::Instance, P: pwm::Instance> {
    melody: Option<&'a Melody>,
    pos: usize,
    volume: u32,
    timer: PlayerTimer<T>,
    buzzer: PlayerBuzzer<P>,
}

impl<'a, T: timer::Instance, P: pwm::Instance> Player<'a, T, P> {
    pub fn new(timer: T, pwm: P, pin: Pin<Output<PushPull>>) -> Self {
        let timer = PlayerTimer::new(timer);
        let buzzer = PlayerBuzzer::new(pwm, pin);

        Player {
            melody: None,
            pos: 0,
            volume: 100,
            timer,
            buzzer,
        }
    }

    pub fn set_volmue(&mut self, volume: u32) {
        self.volume = max(0, min(volume, 100));
        defmt::info!("player::volume {}", self.volume);
    }

    pub fn volume(&self) -> u32 {
        self.volume
    }

    pub fn play(&mut self, melody: &'a Melody) {
        self.melody = Some(melody);
        self.timer.start();
        self.timer.set_play_duration(1.secs()); // play notes after 1 seconds
    }

    pub fn stop(&mut self) {
        self.timer.stop();
        self.buzzer.stop();
        self.melody = None;
    }

    pub fn replay(&mut self) {
        self.timer.stop();
        self.buzzer.stop();
        self.pos = 0;
        self.timer.start();
        self.timer.set_play_duration(1.secs());
    }

    pub fn tick(&mut self) {
        defmt::debug!("player::tick {}", self.timer.now());
        let play_fired = self.timer.check_play();
        let next_fired = self.timer.check_next();
        if let Some(melody) = self.melody {
            if play_fired {
                let buzzer = &self.buzzer;
                let timer = &self.timer;
                let volume = &self.volume;
                if let Some((tone, delay_ms)) = melody.get(self.pos) {
                    defmt::info!(
                        "player::tone: {}, volume: {}%, delay: {}ms",
                        tone,
                        volume,
                        delay_ms
                    );
                    buzzer.tone(tone, volume);
                    timer.set_play_duration((delay_ms * 1_000).micros());
                    timer.set_next_duration((delay_ms * 900).micros());
                } else {
                    defmt::info!("player::replay");
                    self.replay();
                }
            } else if next_fired {
                defmt::info!("player::next_note");
                self.pos += 1;
                self.buzzer.stop();
            }
        }
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

        pub fn tone(&self, tone: Tone, volume: &u32) {
            let max_duty = self.0.set_period(tone.freq()).max_duty();
            let half_max_duty = max_duty as f32 / 2_f32;
            let percent_volmue = *volume as f32 / 100_f32;
            let duty = half_max_duty * percent_volmue;
            self.0.set_duty_on(pwm::Channel::C0, duty as u16);
            self.0.enable();
        }

        pub fn stop(&self) {
            self.0.disable();
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

        #[inline(always)]
        pub fn check_play(&self) -> bool {
            self.check_fired_for_cc(1)
        }

        #[inline(always)]
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
