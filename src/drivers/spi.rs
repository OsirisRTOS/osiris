use crate::error::Result;
use crate::hal;
use crate::sync::once::LazyLock;
use crate::sync::spinlock::SpinLocked;
use crate::types::array::Vec;

// TODO: The 3 is must be a device-tree driven constant.
static BUSES: LazyLock<SpinLocked<Vec<Result<hal::spi::Bus>, 3>>> = LazyLock::new(|| {
    let mut buses = Vec::<Result<hal::spi::Bus>, 3>::new();
    kprint!(
        "Found {} SPI bus entries\n",
        hal::device_tree::SPI_BUS_REGISTRY.len()
    );
    for cfg in hal::device_tree::SPI_BUS_REGISTRY {
        let bus = hal::spi::init(cfg).map_err(|e| e.into());
        match &bus {
            Ok(_) => kprint!("    Initialized SPI bus at 0x{:x}\n", cfg.instance),
            Err(e) => kprint!(
                "    Failed to initialize SPI bus at 0x{:x}: {e}\n",
                cfg.instance
            ),
        }
        buses.push(bus).expect("Bus must fit in inline storage.");
    }
    SpinLocked::new(buses)
});

#[derive(Clone, Copy)]
pub struct Config {
    pub hz: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self { hz: None }
    }
}

pub struct Device {
    desc: hal::spi::Device,
}

impl Device {
    fn new(desc: hal::spi::Device) -> Self {
        Self { desc }
    }
}

impl Device {
    pub fn open(compatible: &str, ordinal: usize, config: Config) -> Result<Self> {
        let dev_cfg = match hal::device_tree::spi_device_by_compatible(compatible, ordinal) {
            Some(cfg) => cfg,
            None => {
                return Err(kerr!(
                    ENODEV,
                    "spi device not found: compatible={compatible}, ordinal={ordinal}"
                ));
            }
        };

        for (i, bus_cfg) in hal::device_tree::SPI_BUS_REGISTRY.iter().enumerate() {
            if bus_cfg.node == dev_cfg.bus_node {
                let buses = BUSES.lock();
                let bus = &buses[i].as_ref().map_err(|e| e.clone())?;
                let desc = hal::spi::init_device(bus, dev_cfg, config.hz)?;
                return Ok(Self::new(desc));
            }
        }

        Err(kerr!(EINVAL))
    }

    pub fn transfer_u8(&self, tx: &[u8], rx: &mut [u8]) -> Result<()> {
        match hal::spi::transfer_words(&self.desc, tx, rx) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn transfer_u16(&self, tx: &[u16], rx: &mut [u16]) -> Result<()> {
        match hal::spi::transfer_words(&self.desc, tx, rx) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        hal::spi::deinit_device(&self.desc);
    }
}

pub fn init() {
    // Force initialization of the BUSES static.
    let _ = LazyLock::force(&BUSES);
}
