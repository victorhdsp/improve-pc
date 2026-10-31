use std::collections::{HashMap, VecDeque};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Local;
use sysinfo::System;

#[cfg(windows)]
use crate::win;

const TICK_SECS: u64 = 1;
const WINDOW: usize = 60;
const STATUS_EVERY: u64 = 10;
const GPU_EVERY: u64 = 3;
#[cfg(windows)]
const DISK_EVERY: u64 = 60;

const RAM_LIMIT: f32 = 85.0;
const CPU_LIMIT: f32 = 90.0;
const TEMP_HIGH: f32 = 85.0;
const TEMP_CRIT: f32 = 92.0;
#[cfg(windows)]
const DISK_FREE_MIN_GB: f64 = 10.0;

#[allow(dead_code)]
struct Sample {
    cpu: f32,
    ram: f32,
    temp_max: Option<f32>,
    pkg_w: Option<f64>,
    gpu: Option<u8>,
}

pub fn run(install: bool) {
    #[cfg(not(windows))]
    let _ = install;

    println!("\n# LOOP — monitor residente (Ctrl+C para sair)");
    println!(
        "limites: ram ≥ {RAM_LIMIT}% | cpu ≥ {CPU_LIMIT}% | temp ≥ {TEMP_HIGH}°C (crítico {TEMP_CRIT}°C) | janela {WINDOW}s"
    );

    let mut sys = System::new();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let cores = sys.cpus().len();
    let vendor = sys
        .cpus()
        .first()
        .map(|c| c.vendor_id().to_string())
        .unwrap_or_default();
    #[cfg(not(windows))]
    let _ = &vendor;
    let brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    let total_ram = sys.total_memory();
    println!(
        "alvo: {} | {} threads | {:.1} GB RAM",
        brand,
        cores,
        total_ram as f64 / 1_073_741_824.0
    );

    #[cfg(windows)]
    let mut msr = match win::msr::MsrCpu::open(&vendor, cores) {
        Ok(m) => Some(m),
        Err(e) if install => {
            println!("driver ausente ({e:?}); tentando instalar...");
            let _ = msr_driver_rs::MsrDriver::install();
            win::msr::MsrCpu::open(&vendor, cores).ok()
        }
        Err(_) => None,
    };
    #[cfg(windows)]
    if msr.is_none() {
        println!("(sem MSR: sem temps de CPU nesta sessão — rode com --install-driver como admin)");
    }

    let gpus = ironmonitor::gpu::GpuCollection::auto_detect().ok();
    if gpus.is_none() {
        println!("(sem backend de GPU)");
    }

    let mut history: VecDeque<Sample> = VecDeque::new();
    let mut cooldown: HashMap<&'static str, Instant> = HashMap::new();
    let mut last_gpu: Option<u8> = None;
    let mut tick: u64 = 0;

    loop {
        thread::sleep(Duration::from_secs(TICK_SECS));
        tick += 1;
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let cpu_use = sys.global_cpu_usage();
        let ram_use = if total_ram > 0 {
            sys.used_memory() as f32 / total_ram as f32 * 100.0
        } else {
            0.0
        };

        #[cfg(windows)]
        let (temp_max, pkg_w) = match msr.as_mut() {
            Some(m) => (m.temp_max_core().or(m.package_temp()), m.package_power()),
            None => (None, None),
        };
        #[cfg(not(windows))]
        let (temp_max, pkg_w): (Option<f32>, Option<f64>) = (None, None);

        if tick % GPU_EVERY == 0 {
            if let Some(g) = &gpus {
                if let Ok(list) = g.snapshot_all() {
                    last_gpu = list.iter().filter_map(|i| i.dynamic_info.utilization).max();
                }
            }
        }

        history.push_back(Sample {
            cpu: cpu_use,
            ram: ram_use,
            temp_max,
            pkg_w,
            gpu: last_gpu,
        });
        while history.len() > WINDOW {
            history.pop_front();
        }

        if tick % STATUS_EVERY == 0 {
            println!(
                "{} | cpu {:.0}% ram {:.0}% | temp {} | pkg {} | gpu {}",
                Local::now().format("%H:%M:%S"),
                cpu_use,
                ram_use,
                fmt_f32(temp_max, "°C"),
                fmt_f64(pkg_w, "W"),
                fmt_u32(last_gpu, "%"),
            );
        }

        check_rules(&history, &mut cooldown);

        #[cfg(windows)]
        if tick % DISK_EVERY == 0 {
            for (dev, free_gb) in win::identify::disks_below(DISK_FREE_MIN_GB) {
                if fire(&mut cooldown, "disk", Duration::from_secs(3600)) {
                    println!(
                        "{} | ALERTA disco: {} com {:.1} GB livres (< {DISK_FREE_MIN_GB} GB)",
                        Local::now().format("%H:%M:%S"),
                        dev,
                        free_gb
                    );
                }
            }
        }
    }
}

fn check_rules(h: &VecDeque<Sample>, cd: &mut HashMap<&'static str, Instant>) {
    if h.len() < WINDOW {
        return;
    }
    let n = h.len() as f32;
    let cpu_avg = h.iter().map(|s| s.cpu).sum::<f32>() / n;
    let ram_avg = h.iter().map(|s| s.ram).sum::<f32>() / n;
    let temps: Vec<f32> = h.iter().filter_map(|s| s.temp_max).collect();
    let now = Local::now().format("%H:%M:%S");

    if ram_avg >= RAM_LIMIT && fire(cd, "ram", Duration::from_secs(300)) {
        println!("{now} | ALERTA RAM: {ram_avg:.0}% na média de {WINDOW}s — pressão de memória sustentada");
    }
    if cpu_avg >= CPU_LIMIT && fire(cd, "cpu", Duration::from_secs(300)) {
        println!("{now} | ALERTA CPU: {cpu_avg:.0}% na média de {WINDOW}s — processador saturado");
    }
    if temps.len() >= WINDOW / 2 {
        let t_avg = temps.iter().sum::<f32>() / temps.len() as f32;
        if t_avg >= TEMP_HIGH && fire(cd, "temp", Duration::from_secs(120)) {
            println!("{now} | ALERTA térmico: {t_avg:.0}°C na média — verificar refrigeração");
        }
    }
    let crit = h
        .iter()
        .rev()
        .take(10)
        .filter_map(|s| s.temp_max)
        .any(|t| t >= TEMP_CRIT);
    if crit && fire(cd, "tempcrit", Duration::from_secs(60)) {
        println!("{now} | CRÍTICO térmico: ≥ {TEMP_CRIT}°C — throttling iminente");
    }
}

fn fire(cd: &mut HashMap<&'static str, Instant>, key: &'static str, cooldown: Duration) -> bool {
    match cd.get(key) {
        Some(t) if t.elapsed() < cooldown => false,
        _ => {
            cd.insert(key, Instant::now());
            true
        }
    }
}

fn fmt_f32(v: Option<f32>, unit: &str) -> String {
    v.map(|x| format!("{x:.0}{unit}")).unwrap_or_else(|| "—".into())
}

fn fmt_f64(v: Option<f64>, unit: &str) -> String {
    v.map(|x| format!("{x:.1}{unit}")).unwrap_or_else(|| "—".into())
}

fn fmt_u32(v: Option<u8>, unit: &str) -> String {
    v.map(|x| format!("{x}{unit}")).unwrap_or_else(|| "—".into())
}
