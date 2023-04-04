use bsp::hal::{
    gpio::{Output, Pin, PushPull},
    pwm, timer,
};

use crate::melody::Melody;

type Instant = fugit::TimerInstantU32<1_000_000>;
type Duration = fugit::TimerDurationU32<1_000_000>;

pub struct Player<'a, T: timer::Instance, P: pwm::Instance> {
    melody: Option<Melody<'a>>,
    timer: T,
    buzzer: pwm::Pwm<P>,
}

impl<'a, T: timer::Instance, P: pwm::Instance> Player<'a, T, P> {
    pub fn new(timer: T, pwm: P, pin: Pin<Output<PushPull>>) -> Self {
        let buzzer = pwm::Pwm::new(pwm);
        buzzer.set_output_pin(pwm::Channel::C0, pin);

        Player {
            melody: None,
            timer,
            buzzer,
        }
    }

    pub fn tick(&mut self) {
        defmt::info!("player::tick");
        if self.check_compare0() {
            let now = self.now();
            defmt::info!("compare0::interrupt {}", now);
        }
    }

    pub fn play(&mut self, melody: Melody<'a>) {
        self.melody = Some(melody);
        self.timer_start();
    }

    pub fn stop(&mut self) {
        self.timer_stop();
    }

    fn timer_start(&self) {
        let timer = self.timer.as_timer0();

        // stop and clear timer
        timer.tasks_stop.write(|w| unsafe { w.bits(1) });
        timer.tasks_clear.write(|w| unsafe { w.bits(1) });

        // set as 32 bits
        timer.bitmode.write(|w| w.bitmode()._32bit());

        // set freqency to 1 Mhz
        timer.prescaler.write(|w| unsafe { w.bits(4) });

        // set compare register (freq / cycles = interrupt freq)
        timer.cc[0].write(|w| unsafe { w.bits(100_000) }); // 100ms

        // enable auto clear
        timer.shorts.write(|w| w.compare0_clear().enabled());

        // enable compare interrupt
        timer.intenset.write(|w| w.compare0().set());

        // start
        timer.tasks_start.write(|w| unsafe { w.bits(1) });
    }

    fn timer_stop(&self) {
        let timer = self.timer.as_timer0();
        timer.tasks_stop.write(|w| unsafe { w.bits(1) });
        timer.tasks_clear.write(|w| unsafe { w.bits(1) });
    }

    #[inline(always)]
    fn now(&self) -> Instant {
        let timer = self.timer.as_timer0();
        timer.tasks_capture[1].write(|w| unsafe { w.bits(1) });
        Instant::from_ticks(timer.cc[1].read().bits())
    }

    #[inline(always)]
    fn check_compare0(&self) -> bool {
        let reg = &self.timer.as_timer0().events_compare[0];
        let fired = reg.read().bits() != 0;
        if fired {
            reg.reset();
        }
        fired
    }
}
