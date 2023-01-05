#![no_std]
#![no_main]

extern crate microbit as hal;

use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = hal::pac, peripherals = true, dispatchers = [])]
mod app {
    use hal::Board;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("init musicbox");

        let Board { buttons, .. } = Board::new(ctx.device, ctx.core);

        let btn1 = buttons.button_a.into_pullup_input().degrade();
        let btn2 = buttons.button_b.into_pullup_input().degrade();

        (Shared {}, Local {}, init::Monotonics())
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }
}
