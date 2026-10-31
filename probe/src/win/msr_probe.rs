use std::thread;
use std::time::Duration;

use msr_driver_rs::MsrDriver;

const INTEL_PLATFORM_INFO: u32 = 0xCE;
const INTEL_THERM_STATUS: u32 = 0x19C;
const INTEL_PKG_THERM_STATUS: u32 = 0x1B2;
const INTEL_PKG_ENERGY: u32 = 0x611;
const INTEL_RAPL_POWER_UNIT: u32 = 0x606;
const AMD_PKG_ENERGY: u32 = 0xC001_029B;

pub fn report(vendor_id: &str, cores: usize, install: bool) {
    println!("\n=== MSR (acesso nativo via driver) ===");
    if install {
        match MsrDriver::install() {
            Ok(()) => println!("driver PawnIO instalado/com atualizado"),
            Err(e) => {
                println!("falha ao instalar (precisa de admin): {e:?}");
                return;
            }
        }
    }
    let driver = match MsrDriver::new() {
        Ok(d) => d,
        Err(e) => {
            println!("indisponível: {e:?}");
            println!("→ rode como admin: probe.exe --install-driver");
            return;
        }
    };

    let v = vendor_id.to_uppercase();
    if v.contains("INTEL") {
        intel(&driver, cores);
    } else if v.contains("AMD") {
        amd(&driver);
    } else {
        println!("vendor desconhecido ({vendor_id}), tentando Intel depois AMD:");
        intel(&driver, cores);
        amd(&driver);
    }
}

fn intel(driver: &MsrDriver, cores: usize) {
    println!("vendor: Intel");
    let tjmax: u32 = match driver.read_msr(INTEL_PLATFORM_INFO, 0) {
        Ok(v) => ((v >> 16) & 0xFF) as u32,
        Err(e) => {
            println!("  TjMax ilegível: {e:?}");
            return;
        }
    };
    println!("  TjMax: {tjmax} °C");

    let mut shown = 0;
    for core in 0..cores as u32 {
        if let Ok(v) = driver.read_msr(INTEL_THERM_STATUS, core) {
            let readout = ((v >> 16) & 0x7F) as u32;
            if readout > 0 {
                println!("  core {core}: {} °C", tjmax.saturating_sub(readout));
                shown += 1;
            }
        }
    }
    if shown == 0 {
        println!("  (nenhuma temperatura por núcleo legível)");
    }
    if let Ok(v) = driver.read_msr(INTEL_PKG_THERM_STATUS, 0) {
        let readout = ((v >> 16) & 0x7F) as u32;
        println!("  package: {} °C", tjmax.saturating_sub(readout));
    }
    println!("  consumo package: {:.1} W", package_power_w(driver, INTEL_PKG_ENERGY, None));
}

fn amd(driver: &MsrDriver) {
    println!("vendor: AMD");
    println!("  (temp de CPU AMD usa SMN via PCI — fora do escopo deste probe)");
    println!(
        "  consumo package: {:.1} W",
        package_power_w(driver, AMD_PKG_ENERGY, Some(65536.0))
    );
}

fn package_power_w(driver: &MsrDriver, msr: u32, fixed_div: Option<f64>) -> f64 {
    let e0 = match driver.read_msr(msr, 0) {
        Ok(v) => v,
        Err(_) => return f64::NAN,
    };
    thread::sleep(Duration::from_secs(1));
    let e1 = match driver.read_msr(msr, 0) {
        Ok(v) => v,
        Err(_) => return f64::NAN,
    };
    let div = match fixed_div {
        Some(d) => d,
        None => {
            let unit = match driver.read_msr(INTEL_RAPL_POWER_UNIT, 0) {
                Ok(u) => u,
                Err(_) => return f64::NAN,
            };
            (1u64 << ((unit >> 8) & 0x1F)) as f64
        }
    };
    e1.wrapping_sub(e0) as f64 / div
}
