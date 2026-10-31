use serde::Deserialize;
use wmi::WMIConnection;

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_ComputerSystem")]
#[serde(rename_all = "PascalCase")]
struct ComputerSystem {
    manufacturer: Option<String>,
    model: Option<String>,
    total_physical_memory: Option<u64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_BaseBoard")]
#[serde(rename_all = "PascalCase")]
struct BaseBoard {
    manufacturer: Option<String>,
    product: Option<String>,
    version: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_BIOS")]
struct Bios {
    manufacturer: Option<String>,
    #[serde(rename = "SMBIOSBIOSVersion")]
    version: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Processor")]
#[serde(rename_all = "PascalCase")]
struct Processor {
    name: Option<String>,
    manufacturer: Option<String>,
    number_of_cores: Option<u32>,
    number_of_logical_processors: Option<u32>,
    max_clock_speed: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_PhysicalMemory")]
#[serde(rename_all = "PascalCase")]
struct PhysicalMemory {
    manufacturer: Option<String>,
    part_number: Option<String>,
    capacity: Option<u64>,
    speed: Option<u32>,
    configured_clock_speed: Option<u32>,
    device_locator: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_VideoController")]
#[serde(rename_all = "PascalCase")]
struct VideoController {
    name: Option<String>,
    driver_version: Option<String>,
    adapter_ram: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_DiskDrive")]
#[serde(rename_all = "PascalCase")]
struct DiskDrive {
    model: Option<String>,
    size: Option<u64>,
    interface_type: Option<String>,
    media_type: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Sensor")]
#[serde(rename_all = "PascalCase")]
struct LhmSensor {
    name: Option<String>,
    value: Option<String>,
    sensor_type: Option<String>,
}

pub fn report() {
    println!("\n=== IDENTIFICAÇÃO (WMI) ===");
    let con = match WMIConnection::new() {
        Ok(c) => c,
        Err(e) => {
            println!("WMI indisponível: {e:?}");
            return;
        }
    };

    match con.query::<ComputerSystem>() {
        Ok(v) => {
            for s in &v {
                println!(
                    "sistema: {:?} {:?} | RAM total: {:?} bytes",
                    s.manufacturer, s.model, s.total_physical_memory
                );
            }
        }
        Err(e) => println!("  [falha Win32_ComputerSystem: {e:?}]"),
    }
    match con.query::<BaseBoard>() {
        Ok(v) => {
            for b in &v {
                println!("placa-mãe: {:?} {:?} (versão {:?})", b.manufacturer, b.product, b.version);
            }
        }
        Err(e) => println!("  [falha Win32_BaseBoard: {e:?}]"),
    }
    match con.query::<Bios>() {
        Ok(v) => {
            for b in &v {
                println!("BIOS: {:?} {:?}", b.manufacturer, b.version);
            }
        }
        Err(e) => println!("  [falha Win32_BIOS: {e:?}]"),
    }
    match con.query::<Processor>() {
        Ok(v) => {
            for p in &v {
                println!(
                    "CPU: {:?} ({:?}) | {:?} núcleos / {:?} threads | {:?} MHz",
                    p.name, p.manufacturer, p.number_of_cores, p.number_of_logical_processors, p.max_clock_speed
                );
            }
        }
        Err(e) => println!("  [falha Win32_Processor: {e:?}]"),
    }
    match con.query::<PhysicalMemory>() {
        Ok(v) => {
            println!("RAM ({} pente(s)):", v.len());
            for m in &v {
                println!(
                    "  slot {:?}: {:?} {:?} | {:?} bytes | {:?} MHz (config {:?} MHz)",
                    m.device_locator, m.manufacturer, m.part_number, m.capacity, m.speed, m.configured_clock_speed
                );
            }
            if v.is_empty() {
                println!("  (nenhum pente listado — RAM provavelmente soldada)");
            }
        }
        Err(e) => println!("  [falha Win32_PhysicalMemory: {e:?}]"),
    }
    match con.query::<VideoController>() {
        Ok(v) => {
            for g in &v {
                println!("GPU: {:?} (driver {:?}, VRAM {:?} bytes)", g.name, g.driver_version, g.adapter_ram);
            }
        }
        Err(e) => println!("  [falha Win32_VideoController: {e:?}]"),
    }
    match con.query::<DiskDrive>() {
        Ok(v) => {
            for d in &v {
                println!("disco: {:?} | {:?} bytes | {:?} / {:?}", d.model, d.size, d.interface_type, d.media_type);
            }
        }
        Err(e) => println!("  [falha Win32_DiskDrive: {e:?}]"),
    }

    println!("\n--- namespaces de monitor externo (fallback de temps) ---");
    for ns in ["ROOT\\OpenHardwareMonitor", "ROOT\\LibreHardwareMonitor"] {
        match WMIConnection::with_namespace_path(ns) {
            Ok(c) => {
                let sensors: Vec<LhmSensor> = c.query().unwrap_or_default();
                println!("{ns}: DISPONÍVEL ({} sensores)", sensors.len());
                for s in sensors.iter().take(10) {
                    println!("  [{:?}] {:?} = {:?}", s.sensor_type, s.name, s.value);
                }
                if sensors.len() > 10 {
                    println!("  ... +{} mais", sensors.len() - 10);
                }
            }
            Err(_) => println!("{ns}: não instalado"),
        }
    }
}
