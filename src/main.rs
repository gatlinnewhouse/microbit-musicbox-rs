#![no_std]
#![no_main]

extern crate microbit as bsp; // board support package

mod button;
mod note;

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = bsp::pac, peripherals = true, dispatchers = [SWI0_EGU0])]
mod app {
    use bsp::hal::clocks::Clocks;
    use bsp::hal::rtc::{Rtc, RtcInterrupt};
    use bsp::pac::RTC0;
    use bsp::Board;

    use crate::button;

    type Button = button::Button<100>; // working frequency 100Hz (rtc0)

    #[shared]
    struct Shared {
        btn1: Button,
        btn2: Button,
    }

    #[local]
    struct Local {
        rtc0: Rtc<RTC0>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("init musicbox");

        let board = Board::new(ctx.device, ctx.core);

        // Starting the low-frequency clock (needed for RTC to work)
        Clocks::new(board.CLOCK).start_lfclk();

        // RTC at 100Hz (32_768 / (327 + 1))
        // 100Hz; 10ms period
        let mut rtc0 = Rtc::new(board.RTC0, 327).unwrap();
        rtc0.enable_event(RtcInterrupt::Tick);
        rtc0.enable_interrupt(RtcInterrupt::Tick, None);
        rtc0.enable_counter();

        // Button A (working rtc0)
        let btn1 = {
            let pin = board.buttons.button_a.into_pullup_input().degrade();
            let mut btn = Button::new(pin);
            btn.attach_event(|event| {
                handle_btn1_event::spawn(event).ok();
            });
            btn
        };

        // Button B (working rtc0)
        let btn2 = {
            let pin = board.buttons.button_b.into_pullup_input().degrade();
            let mut btn = Button::new(pin);
            btn.attach_event(|event| {
                handle_btn2_event::spawn(event).ok();
            });
            btn
        };

        (Shared { btn1, btn2 }, Local { rtc0 }, init::Monotonics())
    }

    #[task(priority = 1, binds = RTC0, local = [rtc0], shared = [btn1, btn2])]
    fn rtc0(mut ctx: rtc0::Context) {
        ctx.local.rtc0.reset_event(RtcInterrupt::Tick);
        ctx.shared.btn1.lock(|btn| btn.tick());
        ctx.shared.btn2.lock(|btn| btn.tick());
    }

    #[task]
    fn handle_btn1_event(_ctx: handle_btn1_event::Context, event: button::Event) {
        defmt::info!("btn1 event: {:?}", &event);
    }

    #[task]
    fn handle_btn2_event(_ctx: handle_btn2_event::Context, event: button::Event) {
        defmt::info!("btn2 event: {:?}", &event);
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}
