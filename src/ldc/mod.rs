//! [`embedded-hal`] driver for Texas Instruments (TI) I2C inductance-to-digital converters (LDC): [LDC1312/LDC1314], [LDC1612/LDC1614].
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal
//! [LDC1312/LDC1314]: https://www.ti.com/lit/ds/symlink/ldc1314.pdf
//! [LDC1612/LDC1614]: https://www.ti.com/lit/ds/symlink/ldc1614.pdf
use embedded_hal::blocking::i2c;

mod data;
pub use data::*;

#[derive(Debug)]
pub enum Error<BE> {
    Bus(BE),
    ConversionUnderRange,
    ConversionOverRange,
    ConversionWatchdogTimeout,
    ConversionAmplitude,
}

/// TI LDC1x1x driver instance
pub struct Ldc<I2c> {
    bus: I2c,
    adr: u8,
}

impl<I2c, BE> Ldc<I2c>
where
    I2c: i2c::Write<Error = BE> + i2c::WriteRead<Error = BE>,
{
    pub fn new(bus: I2c, adr: u8) -> Self {
        Ldc { bus, adr }
    }

    pub fn write_reg(&mut self, reg: u8, data: u16) -> Result<(), Error<BE>> {
        self.bus
            .write(self.adr, &[reg, (data >> 8) as u8, data as u8])
            .map_err(Error::Bus)
    }

    pub fn read_reg(&mut self, reg: u8) -> Result<u16, Error<BE>> {
        let mut result: [u8; 2] = [0xde, 0xad];
        self.bus
            .write_read(self.adr, &[reg], &mut result)
            .map_err(Error::Bus)?;
        Ok((result[0] as u16) << 8 | result[1] as u16)
    }

    /// Read the conversion result for a channel.
    /// Error flags from the result are returned as errors.
    /// Reading does clear the error flags on the device.
    ///
    /// This function must only be used with 12-bit devices (LDC131x).
    /// Use read_data_24bit with 24-bit devices (LDC161x).
    pub fn read_data_12bit(&mut self, ch: Channel) -> Result<u16, Error<BE>> {
        let b = self.read_reg(2 * ch as u8)?;
        if b & (1 << 15) != 0 {
            return Err(Error::ConversionUnderRange);
        }
        if b & (1 << 14) != 0 {
            return Err(Error::ConversionOverRange);
        }
        if b & (1 << 13) != 0 {
            return Err(Error::ConversionWatchdogTimeout);
        }
        if b & (1 << 12) != 0 {
            return Err(Error::ConversionAmplitude);
        }
        Ok(b & 0x0fff)
    }

    /// Read the conversion result for a channel.
    /// Error flags from the result are returned as errors.
    /// Reading does clear the error flags on the device.
    ///
    /// This function must only be used with 24-bit devices (LDC161x).
    /// Use read_data_12bit with 12-bit devices (LDC131x).
    pub fn read_data_24bit(&mut self, ch: Channel) -> Result<u32, Error<BE>> {
        Ok((self.read_data_12bit(ch)? as u32) << 16 | self.read_reg(1 + 2 * ch as u8)? as u32)
    }

    pub fn set_ref_count_conv_interval(&mut self, ch: Channel, intv: u16) -> Result<(), Error<BE>> {
        self.write_reg(0x08 + ch as u8, intv)
    }

    pub fn set_conv_settling_time(&mut self, ch: Channel, cnt: u16) -> Result<(), Error<BE>> {
        self.write_reg(0x10 + ch as u8, cnt)
    }

    pub fn set_clock_dividers(
        &mut self,
        ch: Channel,
        divs: ClockDividers,
    ) -> Result<(), Error<BE>> {
        self.write_reg(0x14 + ch as u8, divs.fin_div << 12 | divs.fref_div)
    }

    pub fn set_error_config(&mut self, conf: ErrorConfig) -> Result<(), Error<BE>> {
        self.write_reg(0x19, conf.0)
    }

    pub fn set_config(&mut self, conf: Config) -> Result<(), Error<BE>> {
        self.write_reg(0x1A, conf.0)
    }

    pub fn set_mux_config(&mut self, conf: MuxConfig) -> Result<(), Error<BE>> {
        self.write_reg(0x1B, conf.0)
    }

    pub fn reset(&mut self) -> Result<(), Error<BE>> {
        self.write_reg(0x1C, 1 << 15)
    }

    // TODO: 131x also have a gain field in the reset register

    pub fn set_sensor_drive_current(&mut self, ch: Channel, cur: u8) -> Result<(), Error<BE>> {
        self.write_reg(0x1E + ch as u8, (cur as u16) << 11)
    }
}
