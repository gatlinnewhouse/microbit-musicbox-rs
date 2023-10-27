#![no_std]
#![no_main]

extern crate microbit as bsp; // board support package

use defmt_rtt as _; // global logger
use panic_probe as _; // panic handler

mod button;
mod melody;
mod mono;
mod player;
mod tone;

#[rtic::app(device = bsp::pac, peripherals = true, dispatchers = [SWI0_EGU0])]
mod app {
    use super::*;

    use bsp::hal::clocks::Clocks;
    use bsp::hal::gpio::{Input, Pin, PullUp};
    use bsp::hal::rtc::{Rtc, RtcInterrupt};
    use bsp::pac::{PWM1, RTC0, TIMER1, TIMER2};
    use bsp::Board;

    type Button = button::Button<Pin<Input<PullUp>>, 100>;
    type Player = player::Player<'static, TIMER1, PWM1>;
    type Display = bsp::display::nonblocking::Display<TIMER2>;

    #[monotonic(binds = TIMER0, default = true)]
    type Mono = mono::MonoTimer<bsp::pac::TIMER0>;

    const MELODY_LIST: &[melody::Melody] = &[
        melody::SUPER_MARIOBROS,
        melody::GAME_OF_THRONES,
        melody::MERRY_CHRISTMAS,
        melody::HAPPY_BIRTHDAY,
        melody::TETRIS,
    ];

    #[shared]
    struct Shared {
        display: Display,
        player: Player,
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
        let mono = mono::MonoTimer::new(board.TIMER0);

        // Starting the low-frequency clock (needed for RTC to work)
        Clocks::new(board.CLOCK).start_lfclk();

        // RTC at 100Hz (32_768 / (327 + 1))
        // 100Hz; 10ms period
        let mut rtc0 = Rtc::new(board.RTC0, 327).unwrap();
        rtc0.enable_event(RtcInterrupt::Tick);
        rtc0.enable_interrupt(RtcInterrupt::Tick, None);
        rtc0.enable_counter();

        // Display
        let display = {
            let pins = board.display_pins;
            Display::new(board.TIMER2, pins)
        };

        // Player
        let player = {
            let pin = board
                .speaker_pin
                .into_push_pull_output(bsp::hal::gpio::Level::High)
                .degrade();
            let mut ply = Player::new(board.TIMER1, board.PWM1, pin, MELODY_LIST);
            ply.play();
            ply
        };

        // Button A
        let btn1 = {
            let pin = board.buttons.button_a.into_pullup_input().degrade();
            let mut btn = Button::new(pin);
            btn.attach_event(|event| {
                handle_btn1_event::spawn(event).ok();
            });
            btn
        };

        // Button B
        let btn2 = {
            let pin = board.buttons.button_b.into_pullup_input().degrade();
            let mut btn = Button::new(pin);
            btn.attach_event(|event| {
                handle_btn2_event::spawn(event).ok();
            });
            btn
        };

        (
            Shared {
                btn1,
                btn2,
                player,
                display,
            },
            Local { rtc0 },
            init::Monotonics(mono),
        )
    }

    #[task(priority = 1, binds = RTC0, local = [rtc0], shared = [player, btn1, btn2])]
    fn rtc0(mut ctx: rtc0::Context) {
        ctx.local.rtc0.reset_event(RtcInterrupt::Tick);
        ctx.shared.btn1.lock(|btn| btn.tick());
        ctx.shared.btn2.lock(|btn| btn.tick());
    }

    #[task(priority = 2, binds = TIMER1, shared = [player])]
    fn timer1(mut ctx: timer1::Context) {
        ctx.shared.player.lock(|ply| ply.handle_play_event());
    }

    #[task(priority = 3, binds = TIMER2, shared = [display])]
    fn timer2(mut ctx: timer2::Context) {
        ctx.shared
            .display
            .lock(|display| display.handle_display_event());
    }

    #[task(shared = [player, display])]
    fn handle_btn1_event(mut ctx: handle_btn1_event::Context, event: button::Event) {
        use button::Event::*;

        defmt::debug!("btn1 event: {:?}", &event);
        ctx.shared.player.lock(|ply| match event {
            Click => ply.volume_sub(10),
            LongPressStart | LongPressDuring | LongPressStop => ply.volume_sub(1),
            DoubleClick => ply.prev(),
            _ => {}
        })
    }

    #[task(shared = [player])]
    fn handle_btn2_event(mut ctx: handle_btn2_event::Context, event: button::Event) {
        use button::Event::*;

        defmt::debug!("btn2 event: {:?}", &event);
        ctx.shared.player.lock(|ply| match event {
            Click => ply.volume_add(10),
            LongPressStart | LongPressDuring | LongPressStop => ply.volume_add(1),
            DoubleClick => ply.next(),
            _ => {}
        })
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}
