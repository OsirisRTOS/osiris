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
    /// Single consumer per controller; concurrent `register_waiter`
    /// calls are rejected with `EBUSY`.
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

    /// Park `uid` as the single waiter on this controller. Returns
    /// `EBUSY` if another thread is already armed — callers must not
    /// share a single CAN device across concurrent receivers.
    pub fn register_waiter(&self, uid: usize) -> Result<()> {
        // `with_bus` yields the inner `arm` Result (kernel `Error`); we
        // collapse both layers into the CAN driver's `PosixError` alias.
        self.with_bus(|bus| bus.waiter.arm(uid))?
            .map_err(|e| e.kind)
    }

    pub fn unregister_waiter(&self) -> Result<()> {
        self.with_bus(|bus| bus.waiter.disarm())
    }

    fn with_bus<R, F: FnOnce(&Bus) -> R>(&self, f: F) -> Result<R> {
        let target_slot = self.desc.index();
        for cell in SLOTS.iter() {
            if let Some(bus) = cell.get() {
                if bus.slot() == target_slot {
                    return Ok(f(bus));
                }
            }
        }
        Err(PosixError::ENODEV)
    }
}

pub fn init() {
    let _ = LazyLock::force(&BUSES);
}

/// Host mirror of the SOF-timestamp wrap-extension in C `drain_fifo()`
/// (`machine/cortex-m/st/stm32l4/interface/can.c`) — keep in sync.
/// Value units: CAN bit-times since boot; wall-time conversion is the
/// consumer's job (divide by bitrate).
#[cfg(test)]
mod hw_ts_extend_spec {
    /// One extension step. `last`/`hi`: persisted per-slot state;
    /// `raw`: new `TIME[15:0]`. Returns `(new_last, new_hi, extended)`.
    fn extend(last: u16, hi: u64, raw: u16) -> (u16, u64, u64) {
        let hi = if raw < last { hi + 0x1_0000 } else { hi };
        (raw, hi, hi | raw as u64)
    }

    /// Walk a sequence of raw readings from zeroed state, as the ISR
    /// does, and collect the extended values.
    fn run(raws: &[u16]) -> Vec<u64> {
        let mut last = 0u16;
        let mut hi = 0u64;
        let mut out = Vec::new();
        for &r in raws {
            let (l, h, ext) = extend(last, hi, r);
            last = l;
            hi = h;
            out.push(ext);
        }
        out
    }

    #[test]
    fn monotonic_within_one_epoch_passes_through() {
        assert_eq!(run(&[0, 1, 100, 5_000, 65_535]), [0, 1, 100, 5_000, 65_535]);
    }

    #[test]
    fn single_wrap_carries_into_high_word() {
        assert_eq!(run(&[65_500, 30]), [65_500, 0x1_0000 + 30]);
    }

    #[test]
    fn many_consecutive_wraps_accumulate() {
        // Each step is below the previous => one wrap per step.
        let v = run(&[60_000, 10, 5, 4, 3]);
        assert_eq!(
            v,
            [
                60_000,
                0x1_0000 + 10,
                0x2_0000 + 5,
                0x3_0000 + 4,
                0x4_0000 + 3
            ]
        );
        // strictly monotonic across wraps
        assert!(v.windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn equal_reading_is_not_treated_as_wrap() {
        // rule is `<`, not `<=`: equality must not bump hi
        assert_eq!(run(&[1234, 1234]), [1234, 1234]);
    }

    #[test]
    fn idle_gap_longer_than_one_epoch_undercounts_is_known_limitation() {
        // Known caveat: a >65.5 ms RX gap hides full wraps (the `<`
        // rule sees only one). Fine — sync traffic is far faster.
        assert_eq!(run(&[100, 90]), [100, 0x1_0000 + 90]);
    }
}
