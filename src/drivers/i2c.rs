use crate::error::Result;
use crate::hal;
use crate::sync::once::LazyLock;
use crate::sync::spinlock::SpinLocked;
use crate::types::array::Vec;

// TODO: The 3 is must be a device-tree driven constant.
static BUSES: LazyLock<SpinLocked<Vec<Result<hal::i2c::Bus>, 3>>> = LazyLock::new(|| {
    let mut buses = Vec::<Result<hal::i2c::Bus>, 3>::new();
    kprint!(
        "Found {} I2C bus entries\n",
        hal::device_tree::I2C_BUS_REGISTRY.len()
    );
    for cfg in hal::device_tree::I2C_BUS_REGISTRY {
        let bus = hal::i2c::init(cfg).map_err(|e| e.into());

        match &bus {
            Ok(_) => kprint!("    Initialized I2C bus at 0x{:x}\n", cfg.instance),
            Err(e) => kprint!(
                "    Failed to initialize I2C bus at 0x{:x}: {e}\n",
                cfg.instance
            ),
        }

        buses.push(bus).expect("Bus must fit in inline storage.");
    }
    SpinLocked::new(buses)
});

pub struct Device {
    desc: hal::i2c::Device,
}

impl Device {
    fn new(desc: hal::i2c::Device) -> Self {
        Self { desc }
    }
}

impl Device {
    pub fn open(compatible: &str, ordinal: usize) -> Result<Self> {
        let dev_cfg = match hal::device_tree::i2c_device_by_compatible(compatible, ordinal) {
            Some(cfg) => cfg,
            None => {
                return Err(kerr!(
                    ENODEV,
                    "i2c device not found: compatible={compatible}, ordinal={ordinal}"
                ));
            }
        };

        for (i, bus_cfg) in hal::device_tree::I2C_BUS_REGISTRY.iter().enumerate() {
            if bus_cfg.node == dev_cfg.bus_node {
                let buses = BUSES.lock();
                let bus = &buses[i].as_ref().map_err(|e| e.clone())?;
                let desc = hal::i2c::init_device(bus, dev_cfg)?;
                return Ok(Self::new(desc));
            }
        }

        Err(kerr!(EINVAL))
    }

    pub fn write(&self, tx: &[u8], timeout: u16) -> Result<()> {
        match hal::i2c::write(&self.desc, tx, timeout) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn read(&self, rx: &mut [u8], timeout: u16) -> Result<()> {
        match hal::i2c::read(&self.desc, rx, timeout) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn write_read(&self, tx: &[u8], rx: &mut [u8], timeout: u16) -> Result<()> {
        match hal::i2c::write_read(&self.desc, tx, rx, timeout) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn check_and_recover_bus(&self) -> Result<bool> {
        match hal::i2c::bus_recovery_needed(&self.desc) {
            Ok(true) => match hal::i2c::recover_bus(&self.desc) {
                Ok(()) => return Ok(true),
                Err(e) => return Err(e.into()),
            },
            Ok(false) => {}
            Err(e) => return Err(e.into()),
        }
        Ok(false)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        hal::i2c::deinit_device(&self.desc);
    }
}

pub fn init() {
    // Force initialization of the BUSES static.
    let _ = LazyLock::force(&BUSES);
}
