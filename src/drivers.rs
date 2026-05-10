pub mod flash;
pub mod i2c;
pub mod spi;

pub fn init() {
    i2c::init();
    spi::init();
}
