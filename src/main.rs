#![no_std]
#![no_main]

extern crate microbit as bsp; // board support package

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = bsp::pac, peripherals = true, dispatchers = [SWI0_EGU0])]
mod app {
    use bsp::{
        hal::{clocks::*, gpio::*, gpiote::*, prelude::*, rtc::*},
        pac::RTC0,
        Board,
    };
    use systick_monotonic::*;

    #[monotonic(binds = SysTick, default = true)]
    type Timer = Systick<100>;

    #[shared]
    struct Shared {
        btn1: Pin<Input<PullUp>>,
        btn2: Pin<Input<PullUp>>,
    }

    #[local]
    struct Local {
        gpiote: Gpiote,
        rtc0: Rtc<RTC0>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::debug!("init musicbox");

        let Board {
            buttons,
            GPIOTE,
            SYST,
            CLOCK,
            RTC0,
            ..
        } = Board::new(ctx.device, ctx.core);

        // Starting the low-frequency clock (needed for RTC to work)
        Clocks::new(CLOCK).start_lfclk();

        // RTC at 100Hz (32_768 / (327 + 1))
        // 100Hz; 10ms period
        let mut rtc0 = Rtc::new(RTC0, 327).unwrap();
        rtc0.enable_event(RtcInterrupt::Tick);
        rtc0.enable_interrupt(RtcInterrupt::Tick, None);
        rtc0.enable_counter();

        let btn1 = buttons.button_a.into_pullup_input().degrade();
        let btn2 = buttons.button_b.into_pullup_input().degrade();

        let gpiote = Gpiote::new(GPIOTE);

        gpiote
            .channel0()
            .input_pin(&btn1)
            .hi_to_lo()
            .enable_interrupt();
        gpiote
            .channel1()
            .input_pin(&btn2)
            .hi_to_lo()
            .enable_interrupt();

        let mono = Systick::new(SYST, 64_000_000);

        (
            Shared { btn1, btn2 },
            Local { gpiote, rtc0 },
            init::Monotonics(mono),
        )
    }

    #[task(binds = GPIOTE, local = [gpiote], shared = [btn1, btn2])]
    fn on_gpiote(ctx: on_gpiote::Context) {
        defmt::debug!("gpiote interrupt");
        ctx.local.gpiote.reset_events();

        let on_gpiote::SharedResources { btn1, btn2 } = ctx.shared;

        (btn1, btn2).lock(|btn1, btn2| {
            let btn1_pressed = btn1.is_low().unwrap();
            let btn2_pressed = btn2.is_low().unwrap();

            match (btn1_pressed, btn2_pressed) {
                (true, true) => defmt::info!("button pressed: A + B"),
                (true, false) => defmt::info!("button pressed: A"),
                (false, true) => defmt::info!("button pressed: B"),
                _ => {}
            }
        });
    }

    #[task(binds = RTC0, priority = 2, local = [rtc0], shared = [btn1, btn2])]
    fn rtc0(ctx: rtc0::Context) {
        defmt::debug!("rtc0 interrupt");
        let rtc0::LocalResources { rtc0 } = ctx.local;

        rtc0.reset_event(RtcInterrupt::Tick);
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}
