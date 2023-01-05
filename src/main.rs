#![no_std]
#![no_main]

extern crate microbit as bsp; // board support package

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = bsp::pac, peripherals = true, dispatchers = [SWI0_EGU0])]
mod app {
    use bsp::hal::prelude::*;
    use bsp::{
        hal::{
            gpio::{Input, Pin, PullUp},
            gpiote::*,
        },
        Board,
    };
    use systick_monotonic::*;

    #[monotonic(binds = SysTick, default = true)]
    type Timer = Systick<100>;

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
        defmt::debug!("init musicbox");

        let Board {
            mut display_pins,
            buttons,
            GPIOTE,
            SYST,
            ..
        } = Board::new(ctx.device, ctx.core);

        let btn1 = buttons.button_a.into_pullup_input().degrade();
        let btn2 = buttons.button_b.into_pullup_input().degrade();

        let _ = display_pins.row3.set_high();
        let led1 = display_pins.col1.degrade();
        let led2 = display_pins.col5.degrade();

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
        gpiote
            .channel2()
            .output_pin(led1)
            .task_out_polarity(TaskOutPolarity::Toggle)
            .init_high();
        gpiote
            .channel3()
            .output_pin(led2)
            .task_out_polarity(TaskOutPolarity::Toggle)
            .init_high();

        let mono = Systick::new(SYST, 64_000_000);

        (
            Shared { gpiote },
            Local { btn1, btn2 },
            init::Monotonics(mono),
        )
    }

    #[task(binds = GPIOTE, shared = [gpiote])]
    fn on_gpiote(mut ctx: on_gpiote::Context) {
        defmt::debug!("gpiote interrupt");
        ctx.shared.gpiote.lock(|gpiote| {
            gpiote.reset_events();

            handle_btn_event::spawn_after(50.millis()).ok();
        });
    }

    #[task(shared = [gpiote], local = [btn1, btn2])]
    fn handle_btn_event(mut ctx: handle_btn_event::Context) {
        let btn1_pressed = ctx.local.btn1.is_low().unwrap();
        let btn2_pressed = ctx.local.btn2.is_low().unwrap();

        ctx.shared
            .gpiote
            .lock(|gpiote| match (btn1_pressed, btn2_pressed) {
                (true, true) => {
                    defmt::info!("A + B");
                    gpiote.channel2().clear();
                    gpiote.channel3().clear();
                }
                (true, false) => {
                    defmt::info!("A");
                    gpiote.channel2().out();
                }
                (false, true) => {
                    defmt::info!("B");
                    gpiote.channel3().out();
                }
                (false, false) => {}
            });
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}
