#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct I2cDeviceRegistryEntry {
    pub bus_node: usize,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct I2cBusRegistryEntry {
    pub node: usize,
    pub instance: usize,
}

pub const I2C_BUS_REGISTRY: &[I2cBusRegistryEntry] = &[];

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SpiDeviceRegistryEntry {
    pub bus_node: usize,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SpiBusRegistryEntry {
    pub node: usize,
    pub instance: usize,
}

pub const SPI_BUS_REGISTRY: &[SpiBusRegistryEntry] = &[];

pub fn i2c_bus_by_dev(_dev: &I2cDeviceRegistryEntry) -> Option<&'static I2cBusRegistryEntry> {
    None
}

pub fn i2c_device_by_compatible(
    _compatible: &str,
    _ord: usize,
) -> Option<&'static I2cDeviceRegistryEntry> {
    None
}

pub fn spi_bus_by_dev(_dev: &SpiDeviceRegistryEntry) -> Option<&'static SpiBusRegistryEntry> {
    None
}

pub fn spi_device_by_compatible(
    _compatible: &str,
    _ord: usize,
) -> Option<&'static SpiDeviceRegistryEntry> {
    None
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CanPin {
    pub port: usize,
    pub line: u8,
    pub af: u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CanIrq {
    pub irqn: u8,
    pub priority: u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CanRegistryEntry {
    pub node: usize,
    pub instance: usize,
    pub bitrate_hz: u32,
    pub rx: CanPin,
    pub tx: CanPin,
    pub rx0_irq: CanIrq,
    pub rx1_irq: CanIrq,
    pub index: u8,
    pub tx_open_drain: u8,
    pub compatible: &'static str,
}

pub const CAN_REGISTRY: &[CanRegistryEntry] = &[];

pub fn can_by_compatible(_compatible: &str, _ord: usize) -> Option<&'static CanRegistryEntry> {
    None
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedDefaultState {
    Off,
    On,
    Keep,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedOutputMode {
    PushPull,
    OpenDrain,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LedRegistryEntry {
    pub node: usize,
    pub port: usize,
    pub line: u8,
    pub active_low: u8,
    pub label: &'static str,
    pub default_state: LedDefaultState,
    pub output_mode: LedOutputMode,
}

pub const LED_REGISTRY: &[LedRegistryEntry] = &[];

pub fn led_by_alias(_alias: &str) -> Option<&'static LedRegistryEntry> {
    None
}

pub fn led_by_label(_label: &str) -> Option<&'static LedRegistryEntry> {
    None
}

pub fn led_by_node(_node: usize) -> Option<&'static LedRegistryEntry> {
    None
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct KeyRegistryEntry {
    pub node: usize,
    pub port: usize,
    pub line: u8,
    pub active_low: u8,
    pub label: &'static str,
    pub code: u32,
    pub debounce_ms: u32,
    pub wakeup_source: u8,
    pub polling_mode: u8,
    pub irq_priority: u8,
}

pub const KEY_REGISTRY: &[KeyRegistryEntry] = &[];

pub fn key_by_alias(_alias: &str) -> Option<&'static KeyRegistryEntry> {
    None
}

pub fn key_by_label(_label: &str) -> Option<&'static KeyRegistryEntry> {
    None
}

pub fn key_by_code(_code: u32) -> Option<&'static KeyRegistryEntry> {
    None
}
