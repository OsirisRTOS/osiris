pub mod can;
pub mod clock;
pub mod i2c;
pub mod key;
pub mod led;
pub mod spi;

pub fn init() {
    clock::init();
    i2c::init();
    spi::init();
    can::init();
    led::init();
    key::init();
}
