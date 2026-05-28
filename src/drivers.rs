pub mod can;
pub mod i2c;
pub mod key;
pub mod led;
pub mod rtc;
pub mod spi;

pub fn init() {
    rtc::init();
    i2c::init();
    spi::init();
    can::init();
    led::init();
    key::init();
}
