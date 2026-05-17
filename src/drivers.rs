pub mod can;
pub mod i2c;
pub mod key;
pub mod led;
pub mod spi;
pub mod uart;

pub fn init() {
    i2c::init();
    spi::init();
    can::init();
    led::init();
    key::init();
}
