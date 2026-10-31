use std::thread;
use std::time::Duration;

use msr_driver_rs::MsrDriver;

use super::msr::MsrCpu;

pub fn report(vendor_id: &str, cores: usize, install: bool) {
    println!("\n=== MSR (acesso nativo via driver) ===");
    if install {
        match MsrDriver::install() {
            Ok(()) => println!("driver PawnIO instalado/atualizado"),
            Err(e) => {
                println!("falha ao instalar (precisa de admin): {e:?}");
                return;
            }
        }
    }
    let mut cpu = match MsrCpu::open(vendor_id, cores) {
        Ok(c) => c,
        Err(e) => {
            println!("indisponível: {e:?}");
            println!("→ rode como admin: probe.exe --install-driver");
            return;
        }
    };

    if cpu.is_intel() {
        println!("vendor: Intel");
        println!(
            "  TjMax: {} °C | crítico: {} °C",
            cpu.tjmax,
            cpu.tjmax.saturating_sub(cpu.tcc_offset)
        );
        for (i, t) in cpu.per_core_temps().into_iter().enumerate() {
            match t {
                Some(t) => println!("  core {i}: {t:.0} °C"),
                None => println!("  core {i}: —"),
            }
        }
        match cpu.package_temp() {
            Some(t) => println!("  package: {t:.0} °C"),
            None => println!("  package: —"),
        }
    } else {
        println!("vendor: AMD");
        println!("  (temp de CPU AMD usa SMN via PCI — fora do escopo deste probe)");
    }

    cpu.package_power();
    thread::sleep(Duration::from_secs(1));
    match cpu.package_power() {
        Some(w) => println!("  consumo package: {w:.1} W"),
        None => println!("  consumo package: indisponível"),
    }
}
