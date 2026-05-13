use crate::error::PosixError;
use crate::hal;
use crate::sync::once::{LazyLock, OnceCell};
use crate::sync::waiter::ParkedWaiter;

pub use hal::can::{BusState, BusStatus, Diag, Filter, Frame, Mode};

pub type Result<T> = core::result::Result<T, PosixError>;

/// Max CAN controllers the kernel tracks; the device tree may populate fewer.
const CAN_BUS_MAX: usize = 2;

pub struct Bus {
    desc: hal::can::Device,
    /// Single consumer per controller — a second `register_waiter` overwrites the first.
    waiter: ParkedWaiter,
}

impl Bus {
    fn slot(&self) -> u8 {
        self.desc.index()
    }
}

/// `OnceCell::set_or_get` writes in place, so the `&'static Bus` it
/// returns stays valid for use as IRQ ctx — unlike values held inside a
/// Vec or returned from a `LazyLock` closure, which move into their
/// final storage.
static SLOTS: [OnceCell<Bus>; CAN_BUS_MAX] = [const { OnceCell::new() }; CAN_BUS_MAX];

#[derive(Clone, Copy)]
struct BusInit {
    slot: u8,
    init: Result<()>,
}

static BUSES: LazyLock<[Option<BusInit>; CAN_BUS_MAX]> = LazyLock::new(|| {
    let mut inits: [Option<BusInit>; CAN_BUS_MAX] = [None; CAN_BUS_MAX];
    kprintln!(
        "Found {} CAN bus entries",
        hal::device_tree::CAN_REGISTRY.len()
    );
    for (i, entry) in hal::device_tree::CAN_REGISTRY.iter().enumerate() {
        if i >= CAN_BUS_MAX {
            kprintln!("    CAN registry exceeds CAN_BUS_MAX={CAN_BUS_MAX}");
            break;
        }
        let bus = Bus {
            desc: hal::can::Device::from_entry(entry),
            waiter: ParkedWaiter::new(),
        };
        let bus_ref: &'static Bus = SLOTS[i].set_or_get(bus);
        // Wire IRQs before `hal::can::init` — it enables interrupts at the
        // chip controller, so any frame landing after must already have a
        // dispatcher in place.
        let init_result =
            wire_irqs(bus_ref).and_then(|()| hal::can::init(&bus_ref.desc, Mode::Normal));
        match init_result {
            Ok(()) => kprintln!("    Initialized CAN bus at 0x{:x}", entry.instance),
            Err(e) => kprintln!(
                "    Failed to initialize CAN bus at 0x{:x}: {:?}",
                entry.instance,
                e,
            ),
        }
        inits[i] = Some(BusInit {
            slot: bus_ref.slot(),
            init: init_result,
        });
    }
    inits
});

fn wire_irqs(bus: &'static Bus) -> Result<()> {
    let ctx = bus as *const Bus as *mut ();
    hal::can::register_irq_handler(&bus.desc, Some(kernel_dispatch), ctx)?;

    let entry = bus.desc.entry();
    let rx0_vector = entry.rx0_irq.irqn as usize + 16;
    let rx1_vector = entry.rx1_irq.irqn as usize + 16;
    let userdata = Some(bus as *const Bus as usize);
    unsafe {
        crate::irq::register_irq(rx0_vector, rx_kernel_handler, userdata)
            .map_err(|_| PosixError::EIO)?;
        crate::irq::register_irq(rx1_vector, rx_kernel_handler, userdata)
            .map_err(|_| PosixError::EIO)?;
    }
    Ok(())
}

extern "C" fn kernel_dispatch(kind: hal::can::Irq, ctx: *mut ()) {
    if !matches!(kind, hal::can::Irq::Rx0 | hal::can::Irq::Rx1) {
        return;
    }
    if ctx.is_null() {
        return;
    }
    // SAFETY: ctx is the `&'static Bus` set by `wire_irqs`, backed by SLOTS.
    let bus = unsafe { &*(ctx as *const Bus) };
    bus.waiter.wake();
}

fn rx_kernel_handler(_ctx: *mut u8, _vector: usize, userdata: Option<usize>) {
    let Some(ptr) = userdata else { return };
    // SAFETY: see `kernel_dispatch`.
    let bus = unsafe { &*(ptr as *const Bus) };
    hal::can::dispatch_isr(bus.slot());
}

pub struct Device {
    desc: hal::can::Device,
}

impl Device {
    pub fn open(compatible: &str, ordinal: usize) -> Result<Self> {
        let _ = LazyLock::force(&BUSES);

        let desc = hal::can::get(compatible, ordinal)?;
        let target_slot = desc.index();
        for entry in BUSES.iter() {
            if let Some(e) = entry {
                if e.slot == target_slot {
                    return match e.init {
                        Ok(()) => Ok(Self { desc }),
                        Err(err) => Err(err),
                    };
                }
            }
        }
        Err(PosixError::ENODEV)
    }

    /// Bring the bus online.
    pub fn start(&self) -> Result<()> {
        hal::can::start(&self.desc)
    }

    pub fn transmit(&self, frame: &Frame) -> Result<()> {
        hal::can::transmit(&self.desc, frame)
    }

    pub fn receive(&self, out: &mut Frame) -> Result<bool> {
        hal::can::receive(&self.desc, out)
    }

    /// Configure a hardware filter. Prefer calling this before `start`.
    pub fn configure_filter(&self, filter: &Filter) -> Result<()> {
        hal::can::configure_filter(&self.desc, filter)
    }

    pub fn bus_status(&self) -> BusStatus {
        hal::can::bus_status(&self.desc)
    }

    pub fn recover(&self) -> Result<()> {
        hal::can::recover(&self.desc)
    }

    pub fn diag(&self) -> Diag {
        hal::can::diag(&self.desc)
    }

    pub fn slot(&self) -> u8 {
        self.desc.index()
    }

    /// Park `uid` as the single waiter on this controller. A second call
    /// overwrites the first.
    pub fn register_waiter(&self, uid: u32) {
        self.with_bus(|bus| bus.waiter.arm(uid));
    }

    pub fn unregister_waiter(&self) {
        self.with_bus(|bus| bus.waiter.disarm());
    }

    fn with_bus<F: FnOnce(&Bus)>(&self, f: F) {
        let target_slot = self.desc.index();
        for cell in SLOTS.iter() {
            if let Some(bus) = cell.get() {
                if bus.slot() == target_slot {
                    f(bus);
                    return;
                }
            }
        }
    }
}

pub fn init() {
    let _ = LazyLock::force(&BUSES);
}
