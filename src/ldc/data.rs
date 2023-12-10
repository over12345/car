#[inline(always)]
const fn sqrt(n: f32) -> f32 {
    // https://bits.stephan-brumme.com/squareRoot.html / no negative check because whatever
    f32::from_bits((n.to_bits() + 0x3f80_0000) >> 1)
}

#[derive(Debug, Clone, Copy)]
pub struct ClockDividers {
    pub fin_div: u16,
    pub fref_div: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct Fsensor(pub f32);

impl Fsensor {
    /// Calculate sensor frequency based on inductance in μH and capacitance in pF.
    pub const fn from_inductance_capacitance(inductance: f32, capacitance: f32) -> Self {
        Self(1.0 / (2.0 * 3.14 * sqrt(inductance * 1e-6_f32 * capacitance * 1e-12_f32)) * 1e-6_f32)
    }

    /// Calculate minimum clock dividers based on Fsensor and Fref (oscillator frequency in MHz).
    ///
    /// If using the internal oscillator, you can pass None, it will default to 43 MHz.
    pub const fn to_clock_dividers(&self, ext_clk_freq: Option<f32>) -> ClockDividers {
        // unwrap_or is not const fn?!
        let fref = match ext_clk_freq {
            None => 43.0, // internal oscillator
            Some(x) => x,
        };
        ClockDividers {
            fin_div: (self.0 / 8.75 + 1.0) as u16,
            fref_div: if self.0 * 4.0 < fref {
                1
            } else if self.0 / 2.0 * 4.0 < fref {
                2
            } else {
                4
            },
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Zero = 0,
    One,
}

#[derive(Debug, Clone, Copy)]
pub struct Status(pub u16);

#[derive(Debug, Default, Clone, Copy)]
pub struct ErrorConfig(pub u16);

impl ErrorConfig {
    #[inline(always)]
    pub const fn with_amplitude_high_error_to_data_register(self, val: bool) -> Self {
        Self(self.0 & !(1 << 12) | (val as u16) << 12)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Config(pub u16);

impl Default for Config {
    fn default() -> Self {
        Self(
            0x1001, /* ch0, only Rp override, reserved first bit 1 */
        )
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Deglitch {
    ThreePointThreeMHz = 0b100,
}

#[derive(Debug, Clone, Copy)]
pub struct MuxConfig(pub u16);

impl Default for MuxConfig {
    fn default() -> Self {
        Self(
            0b0100_0001_111, /* reserved + default 33 MHz deglitch */
        )
    }
}

impl MuxConfig {
    #[inline(always)]
    pub const fn with_auto_scan(self, val: bool) -> Self {
        Self(self.0 & !(1 << 15) | (val as u16) << 15)
    }

    #[inline(always)]
    pub const fn with_deglitch_filter_bandwidth(self, bw: Deglitch) -> Self {
        Self(self.0 & !0b111 | bw as u16)
    }
}
