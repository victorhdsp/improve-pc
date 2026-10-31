use std::time::Instant;

use msr_driver_rs::MsrDriver;

const INTEL_TEMPERATURE_TARGET: u32 = 0x1A2;
const INTEL_THERM_STATUS: u32 = 0x19C;
const INTEL_PKG_THERM_STATUS: u32 = 0x1B2;
const INTEL_PKG_ENERGY: u32 = 0x611;
const INTEL_RAPL_POWER_UNIT: u32 = 0x606;
const AMD_PKG_ENERGY: u32 = 0xC001_029B;

#[derive(Debug)]
#[allow(dead_code)]
pub enum OpenError {
    Driver(msr_driver_rs::Error),
    TemperatureTarget(msr_driver_rs::Error),
}

pub struct MsrCpu {
    driver: MsrDriver,
    intel: bool,
    pub tjmax: u32,
    pub tcc_offset: u32,
    cores: usize,
    energy_div_joules: f64,
    last_energy: Option<(Instant, u64)>,
}

impl MsrCpu {
    pub fn open(vendor_id: &str, cores: usize) -> Result<Self, OpenError> {
        let driver = MsrDriver::new().map_err(OpenError::Driver)?;
        let intel = vendor_id.to_uppercase().contains("INTEL");
        let (tjmax, tcc_offset) = if intel {
            let target = driver
                .read_msr(INTEL_TEMPERATURE_TARGET, 0)
                .map_err(OpenError::TemperatureTarget)?;
            (((target >> 16) & 0xFF) as u32, ((target >> 24) & 0xF) as u32)
        } else {
            (0, 0)
        };
        let energy_div_joules = if intel {
            let unit = driver.read_msr(INTEL_RAPL_POWER_UNIT, 0).unwrap_or(0);
            1.0 / (1u64 << ((unit >> 8) & 0x1F)) as f64
        } else {
            1.0 / 65536.0
        };
        Ok(Self {
            driver,
            intel,
            tjmax,
            tcc_offset,
            cores,
            energy_div_joules,
            last_energy: None,
        })
    }

    pub fn is_intel(&self) -> bool {
        self.intel
    }

    fn core_readout(&self, core: u32) -> Option<u32> {
        let v = self.driver.read_msr(INTEL_THERM_STATUS, core).ok()?;
        let readout = ((v >> 16) & 0x7F) as u32;
        (readout > 0).then_some(readout)
    }

    pub fn per_core_temps(&self) -> Vec<Option<f32>> {
        (0..self.cores as u32)
            .map(|c| self.core_readout(c).map(|r| (self.tjmax.saturating_sub(r)) as f32))
            .collect()
    }

    pub fn temp_max_core(&self) -> Option<f32> {
        self.per_core_temps().into_iter().flatten().fold(None, |m: Option<f32>, t| {
            Some(m.map_or(t, |x| x.max(t)))
        })
    }

    pub fn package_temp(&self) -> Option<f32> {
        if !self.intel {
            return None;
        }
        let v = self.driver.read_msr(INTEL_PKG_THERM_STATUS, 0).ok()?;
        let readout = ((v >> 16) & 0x7F) as u32;
        (readout > 0).then(|| (self.tjmax.saturating_sub(readout)) as f32)
    }

    pub fn package_power(&mut self) -> Option<f64> {
        let msr = if self.intel { INTEL_PKG_ENERGY } else { AMD_PKG_ENERGY };
        let e = self.driver.read_msr(msr, 0).ok()?;
        let now = Instant::now();
        let out = match self.last_energy {
            Some((t, last)) => {
                let dt = now.duration_since(t).as_secs_f64();
                (dt > 0.25).then(|| (e.wrapping_sub(last) as f64) * self.energy_div_joules / dt)
            }
            None => None,
        };
        self.last_energy = Some((now, e));
        out
    }
}
