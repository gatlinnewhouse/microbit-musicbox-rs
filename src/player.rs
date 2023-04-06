use core::ops::Deref;

use bsp::hal::{
    gpio::{Output, Pin, PushPull},
    pwm, timer,
};
use fugit::ExtU32;

use crate::{melody::Melody, tone::Tone};

type Instant = fugit::Instant<u32, 1, 1_000_000>;
type Duration = fugit::Duration<u32, 1, 1_000_000>;

#[derive(Debug)]
enum State<'a> {
    Pending,
    Tone(&'a Melody, usize),
    Wait(&'a Melody, Instant),
    Restart(&'a Melody),
}

pub struct Player<'a, T: timer::Instance, P: pwm::Instance> {
    melody: Option<&'a Melody>,
    pos: usize,
    timer: PlayerTimer<T>,
    buzzer: PlayerBuzzer<P>,
}

impl<'a, T: timer::Instance, P: pwm::Instance> Player<'a, T, P> {
    pub fn new(timer: T, pwm: P, pin: Pin<Output<PushPull>>) -> Self {
        let timer = PlayerTimer::new(timer);
        let buzzer = PlayerBuzzer::new(pwm, pin);

        let player = Player {
            melody: None,
            pos: 0,
            timer,
            buzzer,
        };
        player
    }

    pub fn play(&mut self, melody: &'a Melody) {
        self.melody = Some(melody);
        self.timer.start();
        self.timer.set_compare1(1.secs()); // play notes after 1 seconds
    }

    pub fn stop(&mut self) {
        self.timer.stop();
    }

    pub fn tick(&mut self) {
        let play_fired = self.timer.check_compare1_flag();
        if play_fired {
            let pos = self.pos;
            let melody = self.melody.unwrap();
            let (tone, delay_ms) = melody.get(pos).unwrap();
            self.buzzer.tone(tone, delay_ms);
            self.pos = pos + 1;
        }
    }
}

struct PlayerBuzzer<T: pwm::Instance>(pwm::Pwm<T>);

impl<T: pwm::Instance> PlayerBuzzer<T> {
    pub fn new(pwm: T, pin: Pin<Output<PushPull>>) -> Self {
        let buzzer = pwm::Pwm::new(pwm);
        buzzer.set_output_pin(pwm::Channel::C0, pin);
        PlayerBuzzer(buzzer)
    }

    pub fn tone(&self, tone: Tone, delay_ms: u32) {
        defmt::info!("tone: {}, delay_ms: {}", tone, delay_ms);
    }

}

struct PlayerTimer<T: timer::Instance>(T);

impl<T: timer::Instance> PlayerTimer<T> {
    pub fn new(timer: T) -> Self {
        let timer = PlayerTimer(timer);
        timer.tasks_stop.write(|w| w.tasks_stop().set_bit());
        timer.tasks_clear.write(|w| w.tasks_clear().set_bit());
        timer.bitmode.write(|w| w.bitmode()._32bit());
        timer.prescaler.write(|w| unsafe { w.prescaler().bits(4) }); // 1 Mhz
        timer
    }

    #[inline(always)]
    pub fn start(&self) {
        self.tasks_start.write(|w| unsafe { w.bits(1) });
    }

    #[inline(always)]
    pub fn stop(&self) {
        self.tasks_stop.write(|w| unsafe { w.bits(1) });
        self.tasks_clear.write(|w| unsafe { w.bits(1) });
    }

    fn set_compare1(&mut self, duration: Duration) {
        let now = self.now();
        let instant = now + duration;
        self.cc[1].write(|w| unsafe { w.cc().bits(instant.duration_since_epoch().ticks()) });
    }

    fn check_compare1_flag(&mut self) -> bool {
        let reg = &self.events_compare[1];
        let fired = reg.read().bits() != 0;
        if fired {
            reg.reset();
        }
        fired
    }

    #[inline(always)]
    pub fn now(&self) -> Instant {
        self.tasks_capture[0].write(|w| unsafe { w.bits(1) });
        Instant::from_ticks(self.cc[0].read().bits())
    }
}

impl<T: timer::Instance> Deref for PlayerTimer<T> {
    type Target = bsp::pac::timer0::RegisterBlock;

    fn deref(&self) -> &Self::Target {
        self.0.as_timer0()
    }
}
