#![no_std]
#![no_main]

extern crate microbit as bsp; // board support package
extern crate nrf52833_hal as hal; // hardware abstraction layer

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = bsp::pac, peripherals = true, dispatchers = [SWI0_EGU0])]
mod app {
    use bsp::{
        hal::{
            gpio::{Input, Pin, PullUp},
            gpiote::Gpiote,
        },
        Board,
    };
    use hal::prelude::*;

    #[shared]
    struct Shared {
        gpiote: Gpiote,
    }

    #[local]
    struct Local {
        btn1: Pin<Input<PullUp>>,
        btn2: Pin<Input<PullUp>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("init musicbox");

        let Board {
            buttons, GPIOTE, ..
        } = Board::new(ctx.device, ctx.core);

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

        (Shared { gpiote }, Local { btn1, btn2 }, init::Monotonics())
    }

    #[task(binds = GPIOTE, shared = [gpiote])]
    fn on_gpiote(mut ctx: on_gpiote::Context) {
        defmt::info!("gpiote interrupt");
        ctx.shared.gpiote.lock(|gpiote| {
            gpiote.reset_events();

            handle_btn_event::spawn().unwrap();
        });
    }

    #[task(local = [btn1, btn2])]
    fn handle_btn_event(ctx: handle_btn_event::Context) {
        let btn1_pressed = ctx.local.btn1.is_low().unwrap();
        let btn2_pressed = ctx.local.btn2.is_low().unwrap();

        match (btn1_pressed, btn2_pressed) {
            (true, true) => {
                defmt::info!("A + B");
            }
            (true, false) => {
                defmt::info!("A");
            }
            (false, true) => {
                defmt::info!("B");
            }
            (false, false) => {}
        }
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }
}
