#![no_main]
#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};
use defmt_rtt as _; // global logger
use hal::gpio::{EPin, Input};
use hal::otg_fs::{UsbBusType, USB};
use hal::prelude::*;
use hal::serial;
use keyberon::debounce::Debouncer;
use keyberon::key_code::KbHidReport;
use keyberon::layout::{CustomEvent, Event, Layout};
use keyberon::matrix::DirectPinMatrix;
use nb::block;
use stm32f4xx_hal as hal;
use usb_device::bus::UsbBusAllocator;
use usb_device::class::UsbClass as _;
use usb_device::device::UsbDeviceState;
use usb_device::prelude::*;

use panic_probe as _;

pub mod layout;

/// USB VIP for a generic keyboard from
/// https://github.com/obdev/v-usb/blob/master/usbdrv/USB-IDs-for-free.txt
const VID: u16 = 0x16c0;

/// USB PID for a generic keyboard from
/// https://github.com/obdev/v-usb/blob/master/usbdrv/USB-IDs-for-free.txt
const PID: u16 = 0x27db;

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", {
    // NOTE(no-CAS) `timestamps` runs with interrupts disabled
    let n = COUNT.load(Ordering::Relaxed);
    COUNT.store(n + 1, Ordering::Relaxed);
    n
});

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

type UsbClass = keyberon::Class<'static, UsbBusType, ()>;
type UsbDevice = usb_device::device::UsbDevice<'static, UsbBusType>;

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers=[TIM1_CC])]
mod app {
    use super::*;
    // shared resources (between tasks)
    #[shared]
    struct Shared {
        usb_dev: UsbDevice,
        usb_class: UsbClass,
        #[lock_free]
        layout: Layout<12, 4, 5, ()>,
    }

    // local resources (between tasks)
    #[local]
    struct Local {
        matrix: DirectPinMatrix<EPin<Input>, 6, 4>,
        debouncer: Debouncer<[[bool; 6]; 4]>,
        timer: hal::timer::counter::CounterHz<hal::pac::TIM2>,
        serial_tx: serial::Tx<hal::pac::USART1>,
        serial_rx: serial::Rx<hal::pac::USART1>,
        serial_buf: [u8; 4],
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("init");
        // prepare static datastructures for USB
        static mut EP_MEMORY: [u32; 1024] = [0; 1024];
        static mut USB_BUS: Option<UsbBusAllocator<UsbBusType>> = None;

        // setup the monotonic timer
        let mut clocks = cx
            .device
            .RCC
            .constrain()
            .cfgr
            .use_hse(25.MHz())
            .sysclk(84.MHz())
            .require_pll48clk()
            .freeze();

        // get GPIO pins
        let gpioa = cx.device.GPIOA.split();
        let gpiob = cx.device.GPIOB.split();

        // timer for processing keyboard events and sending a USB keyboard report
        let mut timer = cx.device.TIM2.counter_hz(&mut clocks);
        // or equivalently
        // let mut timer = hal::timer::Timer::new(cx.device.TIM2, &mut clocks).counter_hz();
        timer.start(1.kHz()).unwrap();
        timer.listen(hal::timer::Event::Update);

        // initialize USB
        let usb = USB {
            usb_global: cx.device.OTG_FS_GLOBAL,
            usb_device: cx.device.OTG_FS_DEVICE,
            usb_pwrclk: cx.device.OTG_FS_PWRCLK,
            pin_dm: gpioa.pa11.into_alternate().into(),
            pin_dp: gpioa.pa12.into_alternate().into(),
            hclk: clocks.hclk(),
        };

        unsafe {
            USB_BUS = Some(UsbBusType::new(usb, &mut EP_MEMORY));
        }

        let usb_bus = unsafe { USB_BUS.as_ref().unwrap() };
        let usb_class = keyberon::new_class(&usb_bus, ());
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(VID, PID))
            .manufacturer("Dario Götz")
            .product("Dario Götz's 42-key split keyboard")
            .serial_number(env!("CARGO_PKG_VERSION"))
            .build();

        // Setup USART communication with other half
        let (pb6, pb7) = (gpiob.pb6, gpiob.pb7);
        let serial_pins = cortex_m::interrupt::free(move |_cs| {
            (pb6.into_alternate::<7>(), pb7.into_alternate::<7>())
        });
        let mut serial = cx
            .device
            .USART1
            .serial(
                serial_pins,
                hal::serial::config::Config::default().baudrate(38_400.bps()),
                &clocks,
            )
            .unwrap();
        // or equivalently
        // let mut serial = serial::Serial::new(cx.device.USART1, pins, 38_400.bps(), &mut clocks);
        serial.listen(serial::Event::Rxne);
        let (serial_tx, serial_rx) = serial.split();

        // define pin to matrix relation (prepare outside of interrupt::free closure
        // due to gpioa/gpiob move)
        let matrix_pins = [
            [
                Some(gpiob.pb1.into_pull_up_input().erase()),
                Some(gpiob.pb10.into_pull_up_input().erase()),
                Some(gpioa.pa8.into_pull_up_input().erase()),
                Some(gpiob.pb15.into_pull_up_input().erase()),
                Some(gpiob.pb14.into_pull_up_input().erase()),
                Some(gpiob.pb13.into_pull_up_input().erase()),
            ],
            [
                Some(gpiob.pb9.into_pull_up_input().erase()),
                Some(gpiob.pb8.into_pull_up_input().erase()),
                Some(gpiob.pb5.into_pull_up_input().erase()),
                Some(gpiob.pb4.into_pull_up_input().erase()),
                Some(gpiob.pb3.into_pull_up_input().erase()),
                Some(gpioa.pa15.into_pull_up_input().erase()),
            ],
            [
                Some(gpioa.pa3.into_pull_up_input().erase()),
                Some(gpioa.pa4.into_pull_up_input().erase()),
                Some(gpioa.pa5.into_pull_up_input().erase()),
                Some(gpioa.pa6.into_pull_up_input().erase()),
                Some(gpioa.pa7.into_pull_up_input().erase()),
                Some(gpiob.pb0.into_pull_up_input().erase()),
            ],
            [
                None,
                None,
                None,
                Some(gpioa.pa2.into_pull_up_input().erase()),
                Some(gpioa.pa1.into_pull_up_input().erase()),
                Some(gpioa.pa0.into_pull_up_input().erase()),
            ],
        ];
        let matrix = cortex_m::interrupt::free(move |_cs| DirectPinMatrix::new(matrix_pins));

        let mut layout = Layout::new(&layout::LAYERS);
        layout.add_tri_state_layer((1, 2), 3);

        (
            Shared {
                // Initialization of shared resources go here
                usb_dev,
                usb_class,
                layout,
            },
            Local {
                // Initialization of local resources go here
                matrix: matrix.unwrap(),
                timer,
                debouncer: Debouncer::new([[false; 6]; 4], [[false; 6]; 4], 5),
                serial_tx,
                serial_rx,
                serial_buf: [0; 4],
            },
            init::Monotonics(),
        )
    }

    // Optional idle, can be removed if not needed.
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            continue;
        }
    }

    /// Register a key press/release event with the layout (it will not be processed, yet)
    #[task(priority=1, capacity=8, shared=[layout])]
    fn register_keyboard_event(cx: register_keyboard_event::Context, event: Event) {
        // match event {
        //     Event::Press(i, j) => defmt::info!("Registering press {} {}", i, j),
        //     Event::Release(i, j) => defmt::info!("Registering release {} {}", i, j),
        // }
        cx.shared.layout.event(event)
    }

    /// Check all switches for their state, register corresponding events, and
    /// spawn generation of a USB keyboard report (including layout event processing)
    #[task(binds=TIM2, priority=1, local=[debouncer, matrix, timer, serial_tx], shared=[usb_dev, usb_class, layout])]
    fn tick(mut cx: tick::Context) {
        // defmt::info!("Processing keyboard events");
        let is_host = cx.shared.usb_dev.lock(|d| d.state()) == UsbDeviceState::Configured;

        cx.local.timer.wait().ok();
        // or equivalently
        // cx.local.timer.clear_interrupt(hal::timer::Event::Update);

        // scan keyboard
        for event in cx
            .local
            .debouncer
            .events(cx.local.matrix.get().unwrap())
            .map(transform_keypress_coordinates)
        {
            // either register events or send to other half
            if is_host {
                cx.shared.layout.event(event)
            } else {
                for &b in &serialize(event) {
                    block!(cx.local.serial_tx.write(b)).unwrap();
                }
            }
            // match event {
            //     Event::Press(i, j) => defmt::info!("Pressed {} {}", i, j),
            //     Event::Release(i, j) => defmt::info!("Released {} {}", i, j),
            // }
        }

        let tick = cx.shared.layout.tick();
        match tick {
            CustomEvent::Release(()) => unsafe { cortex_m::asm::bootload(0x1FFF0000 as _) },
            _ => (),
        }

        // if this is the USB-side, send a USB keyboard report
        if is_host {
            let report: KbHidReport = cx.shared.layout.keycodes().collect();
            if cx
                .shared
                .usb_class
                .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
            {
                while let Ok(0) = cx.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
            }
        }
    }

    /// Receive USART events from other keyboard half and register them
    #[task(binds = USART1, priority = 2, local = [serial_rx, serial_buf])]
    fn rx(cx: rx::Context) {
        // receive USART bytes and place into local buffer
        // if buffer is full (ends with '\n'), spawn event registration
        // received events (from other half) are mirrored (transformed)
        if let Ok(b) = cx.local.serial_rx.read() {
            cx.local.serial_buf.rotate_left(1);
            cx.local.serial_buf[3] = b;

            if cx.local.serial_buf[3] == b'\n' {
                if let Ok(event) = deserialize(&cx.local.serial_buf[..]) {
                    defmt::info!("Received message via USART");
                    register_keyboard_event::spawn(event).unwrap()
                }
            }
        }
    }

    fn deserialize(bytes: &[u8]) -> Result<Event, ()> {
        match *bytes {
            [b'P', i, j, b'\n'] => Ok(Event::Press(i, j)),
            [b'R', i, j, b'\n'] => Ok(Event::Release(i, j)),
            _ => Err(()),
        }
    }

    fn serialize(e: Event) -> [u8; 4] {
        match e {
            Event::Press(i, j) => [b'P', i, j, b'\n'],
            Event::Release(i, j) => [b'R', i, j, b'\n'],
        }
    }

    /// Transform key events from other keyboard half by mirroring coordinates
    #[cfg(feature = "right_half")]
    fn transform_keypress_coordinates(e: Event) -> Event {
        // mirror coordinates for events for right half
        e.transform(|i, j| (i, 11 - j))
    }

    #[cfg(not(feature = "right_half"))]
    fn transform_keypress_coordinates(e: Event) -> Event {
        e
    }

    // USB events
    #[task(binds = OTG_FS, priority = 3, shared = [usb_dev, usb_class])]
    fn usb_tx(cx: usb_tx::Context) {
        (cx.shared.usb_dev, cx.shared.usb_class).lock(|mut usb_dev, mut usb_class| {
            usb_poll(&mut usb_dev, &mut usb_class);
        });
    }

    #[task(binds = OTG_FS_WKUP, priority = 3, shared = [usb_dev, usb_class])]
    fn usb_rx(cx: usb_rx::Context) {
        (cx.shared.usb_dev, cx.shared.usb_class).lock(|mut usb_dev, mut usb_class| {
            usb_poll(&mut usb_dev, &mut usb_class);
        });
    }

    fn usb_poll(usb_dev: &mut UsbDevice, keyboard: &mut UsbClass) {
        if usb_dev.poll(&mut [keyboard]) {
            keyboard.poll();
        }
    }
}
