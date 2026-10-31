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

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_LogicalDisk")]
#[serde(rename_all = "PascalCase")]
struct LogicalDisk {
    device_id: Option<String>,
    drive_type: Option<u32>,
    size: Option<u64>,
    free_space: Option<u64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_NetworkAdapter")]
#[serde(rename_all = "PascalCase")]
struct NetworkAdapter {
    net_connection_id: Option<String>,
    adapter_type: Option<String>,
    speed: Option<u64>,
    physical_adapter: Option<bool>,
    net_enabled: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Battery")]
#[serde(rename_all = "PascalCase")]
struct Battery {
    estimated_charge_remaining: Option<u16>,
    battery_status: Option<u16>,
}

#[repr(C)]
struct SystemBatteryState {
    ac_on_line: u8,
    battery_present: u8,
    charging: u8,
    discharging: u8,
    spare: [u8; 4],
    rate_mw: u32,
    capacity_mwh: u32,
    voltage_mv: u32,
    charge_rate_mw: u32,
    discharge_rate_mw: u32,
    default_alert1: u32,
    default_alert2: u32,
    default_alert3: u32,
}

fn battery_power_state() -> Option<(bool, bool)> {
    let mut s: SystemBatteryState = unsafe { std::mem::zeroed() };
    let status = unsafe {
        windows_sys::Win32::System::Power::CallNtPowerInformation(
            5,
            std::ptr::null(),
            0,
            &mut s as *mut SystemBatteryState as *mut core::ffi::c_void,
            std::mem::size_of::<SystemBatteryState>() as u32,
        )
    };
    (status == 0 && s.battery_present != 0).then(|| (s.ac_on_line != 0, s.charging != 0))
}

fn fmt_link_bps(bps: Option<u64>) -> String {
    match bps {
        Some(v) if v >= 1_000_000_000 => format!("{:.1} Gbps", v as f64 / 1e9),
        Some(v) if v >= 1_000_000 => format!("{:.0} Mbps", v as f64 / 1e6),
        Some(v) => format!("{v} bps"),
        None => "?".into(),
    }
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

    println!("\n--- rede e energia ---");
    match con.query::<NetworkAdapter>() {
        Ok(v) => {
            let fisicos: Vec<_> = v.iter().filter(|a| a.physical_adapter == Some(true)).collect();
            if fisicos.is_empty() {
                println!("rede: nenhum adaptador físico listado");
            }
            for a in fisicos {
                println!(
                    "rede: {:?} | {:?} | link {} | {}",
                    a.net_connection_id,
                    a.adapter_type,
                    fmt_link_bps(a.speed),
                    if a.net_enabled == Some(true) { "ativo" } else { "inativo" }
                );
            }
        }
        Err(e) => println!("  [falha Win32_NetworkAdapter: {e:?}]"),
    }
    match con.query::<Battery>() {
        Ok(v) if !v.is_empty() => {
            let pwr = battery_power_state();
            for b in &v {
                let estado = match pwr {
                    Some((true, true)) => "na tomada, carregando",
                    Some((true, false)) => "na tomada",
                    Some((false, _)) => "na bateria",
                    None => "estado: ?",
                };
                println!(
                    "bateria: {:?}% | {} | status WMI: {:?}",
                    b.estimated_charge_remaining, estado, b.battery_status
                );
            }
        }
        Ok(_) => println!("bateria: nenhuma (desktop)"),
        Err(e) => println!("  [falha Win32_Battery: {e:?}]"),
    }

    println!("\n--- namespaces de monitor externo (fallback de temps) ---");    for ns in ["ROOT\\OpenHardwareMonitor", "ROOT\\LibreHardwareMonitor"] {
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

pub fn usage_report() {
    println!("\n=== USO ATUAL (perf counters) ===");
    let con = match WMIConnection::new() {
        Ok(c) => c,
        Err(e) => {
            println!("WMI indisponível: {e:?}");
            return;
        }
    };

    match con.query::<LogicalDisk>() {
        Ok(v) => {
            for d in v.iter().filter(|d| d.drive_type == Some(3)) {
                match (d.device_id.as_deref(), d.size, d.free_space) {
                    (Some(id), Some(s), Some(f)) if s > 0 => {
                        let used = ((s - f) as f64 / s as f64) * 100.0;
                        println!("disco {}: {:.0}% usado ({:.0} GB livres)", id, used, f as f64 / 1e9);
                    }
                    (Some(id), _, _) => println!("disco {}: sem info de espaço", id),
                    _ => {}
                }
            }
        }
        Err(e) => println!("  [falha Win32_LogicalDisk: {e:?}]"),
    }
}

pub fn disks_below(min_free_gb: f64) -> Vec<(String, f64)> {
    let con = match WMIConnection::new() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    match con.query::<LogicalDisk>() {
        Ok(v) => v
            .iter()
            .filter(|d| d.drive_type == Some(3))
            .filter_map(|d| {
                let free = d.free_space? as f64 / 1e9;
                (free < min_free_gb)
                    .then(|| (d.device_id.clone().unwrap_or_else(|| "?".into()), free))
            })
            .collect(),
        Err(_) => vec![],
    }
}
