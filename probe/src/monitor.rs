use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Local;
use sysinfo::System;

#[cfg(windows)]
use crate::win;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[allow(dead_code)]
pub enum Componente {
    Placa,
    Cpu,
    Ram,
    Gpu,
    Disco,
    Rede,
    Bateria,
}

impl Componente {
    pub fn nome(self) -> &'static str {
        match self {
            Componente::Placa => "Placa-mãe",
            Componente::Cpu => "Processador",
            Componente::Ram => "Memória RAM",
            Componente::Gpu => "Placa de vídeo",
            Componente::Disco => "Armazenamento",
            Componente::Rede => "Rede",
            Componente::Bateria => "Bateria",
        }
    }

    #[allow(dead_code)]
    pub fn todos() -> [Componente; 7] {
        [
            Componente::Placa,
            Componente::Cpu,
            Componente::Ram,
            Componente::Gpu,
            Componente::Disco,
            Componente::Rede,
            Componente::Bateria,
        ]
    }
}

#[derive(Clone)]
pub struct Alerta {
    pub componente: Componente,
    pub msg: String,
    pub hora: String,
}

#[derive(Clone)]
pub struct Instantaneo {
    pub cpu: f32,
    pub ram: f32,
    pub ram_usada_gb: f64,
    pub ram_total_gb: f64,
    pub temp: Option<f32>,
    pub pkg_w: Option<f64>,
    pub gpu: Option<u8>,
    pub gpu_mem_gb: Option<f64>,
}

#[derive(Clone, Default)]
pub struct MonitorState {
    pub inst: Option<Instantaneo>,
    pub cpu_media: f32,
    pub ram_media: f32,
    pub temp_media: Option<f32>,
    pub ativos: Vec<Componente>,
    pub alertas: VecDeque<Alerta>,
    pub log: VecDeque<String>,
}

pub type Shared = Arc<Mutex<MonitorState>>;

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

#[derive(Clone, Copy)]
struct Sample {
    cpu: f32,
    ram: f32,
    temp: Option<f32>,
}

fn logar(st: &mut MonitorState, linha: String) {
    st.log.push_back(linha);
    while st.log.len() > 300 {
        st.log.pop_front();
    }
}

pub fn iniciar(estado: Shared) {
    thread::spawn(move || sampler(estado));
}

fn sampler(estado: Shared) {
    let mut sys = System::new();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    #[cfg(windows)]
    let cores = sys.cpus().len();
    #[cfg(not(windows))]
    let _cores = 0;
    #[cfg(windows)]
    let vendor = sys
        .cpus()
        .first()
        .map(|c| c.vendor_id().to_string())
        .unwrap_or_default();
    #[cfg(not(windows))]
    let _vendor = String::new();
    let total_ram = sys.total_memory();

    #[cfg(windows)]
    let mut msr = match win::msr::MsrCpu::open(&vendor, cores) {
        Ok(m) => Some(m),
        Err(e) if win::admin::is_elevated() => {
            let _ = msr_driver_rs::MsrDriver::install();
            let reaberto = win::msr::MsrCpu::open(&vendor, cores);
            if reaberto.is_err() {
                if let Ok(mut st) = estado.lock() {
                    logar(&mut st, format!("driver MSR indisponível ({e:?}) — rode como admin uma vez"));
                }
            }
            reaberto.ok()
        }
        Err(_) => None,
    };

    let gpus = ironmonitor::gpu::GpuCollection::auto_detect().ok();

    let mut history: VecDeque<Sample> = VecDeque::new();
    let mut cooldown: HashMap<&'static str, Instant> = HashMap::new();
    let mut last_gpu: Option<u8> = None;
    let mut last_gpu_mem_gb: Option<f64> = None;
    let mut tick: u64 = 0;
    #[cfg(windows)]
    let mut disco_ativo_ate = Instant::now() - Duration::from_secs(1);

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
                    last_gpu_mem_gb = list
                        .iter()
                        .filter_map(|i| i.dynamic_info.memory.used)
                        .max()
                        .map(|b| b as f64 / 1_073_741_824.0);
                }
            }
        }

        history.push_back(Sample {
            cpu: cpu_use,
            ram: ram_use,
            temp: temp_max,
        });
        while history.len() > WINDOW {
            history.pop_front();
        }

        let n = history.len() as f32;
        let cpu_media = history.iter().map(|s| s.cpu).sum::<f32>() / n;
        let ram_media = history.iter().map(|s| s.ram).sum::<f32>() / n;
        let temps: Vec<f32> = history.iter().filter_map(|s| s.temp).collect();
        let temp_media = if temps.is_empty() {
            None
        } else {
            Some(temps.iter().sum::<f32>() / temps.len() as f32)
        };

        let hora = Local::now().format("%H:%M:%S").to_string();

        let mut novo_estado = MonitorState {
            inst: Some(Instantaneo {
                cpu: cpu_use,
                ram: ram_use,
                ram_usada_gb: sys.used_memory() as f64 / 1_073_741_824.0,
                ram_total_gb: total_ram as f64 / 1_073_741_824.0,
                temp: temp_max,
                pkg_w,
                gpu: last_gpu,
                gpu_mem_gb: last_gpu_mem_gb,
            }),
            cpu_media,
            ram_media,
            temp_media,
            ativos: Vec::new(),
            alertas: VecDeque::new(),
            log: VecDeque::new(),
        };

        let pega = |cd: &mut HashMap<&'static str, Instant>, k: &'static str, cd_secs: u64| -> bool {
            match cd.get(k) {
                Some(t) if t.elapsed() < Duration::from_secs(cd_secs) => false,
                _ => {
                    cd.insert(k, Instant::now());
                    true
                }
            }
        };

        let mut disparos: Vec<(Componente, String)> = Vec::new();
        if history.len() >= WINDOW {
            if ram_media >= RAM_LIMIT && pega(&mut cooldown, "ram", 300) {
                disparos.push((
                    Componente::Ram,
                    format!("RAM em {ram_media:.0}% na média de {WINDOW}s — pressão de memória sustentada"),
                ));
            }
            if cpu_media >= CPU_LIMIT && pega(&mut cooldown, "cpu", 300) {
                disparos.push((
                    Componente::Cpu,
                    format!("CPU em {cpu_media:.0}% na média de {WINDOW}s — processador saturado"),
                ));
            }
            if temps.len() >= WINDOW / 2 && temp_media.unwrap_or(0.0) >= TEMP_HIGH && pega(&mut cooldown, "temp", 120) {
                disparos.push((
                    Componente::Cpu,
                    format!("temperatura em {:.0}°C na média — verificar refrigeração", temp_media.unwrap()),
                ));
            }
            let crit = history.iter().rev().take(10).filter_map(|s| s.temp).any(|t| t >= TEMP_CRIT);
            if crit && pega(&mut cooldown, "tempcrit", 60) {
                disparos.push((Componente::Cpu, format!("CRÍTICO: ≥ {TEMP_CRIT}°C — throttling iminente")));
            }
        }

        if ram_media >= RAM_LIMIT {
            novo_estado.ativos.push(Componente::Ram);
        }
        if cpu_media >= CPU_LIMIT || temp_media.unwrap_or(0.0) >= TEMP_HIGH {
            novo_estado.ativos.push(Componente::Cpu);
        }

        #[cfg(windows)]
        if tick % DISK_EVERY == 0 {
            let baixos = win::identify::disks_below(DISK_FREE_MIN_GB);
            if !baixos.is_empty() {
                disco_ativo_ate = Instant::now() + Duration::from_secs(3600);
            }
            for (dev, free_gb) in baixos {
                if pega(&mut cooldown, "disk", 3600) {
                    disparos.push((
                        Componente::Disco,
                        format!("{dev} com {free_gb:.1} GB livres (< {DISK_FREE_MIN_GB} GB)"),
                    ));
                }
            }
        }
        #[cfg(windows)]
        if Instant::now() < disco_ativo_ate {
            novo_estado.ativos.push(Componente::Disco);
        }

        if tick % STATUS_EVERY == 0 {
            let fmt = |v: Option<f32>, u: &str| v.map(|x| format!("{x:.0}{u}")).unwrap_or_else(|| "—".into());
            logar(
                &mut novo_estado,
                format!(
                    "{hora} | cpu {:.0}% ram {:.0}% | temp {} | pkg {} | gpu {}",
                    cpu_use,
                    ram_use,
                    fmt(temp_max, "°C"),
                    pkg_w.map(|x| format!("{x:.1}W")).unwrap_or_else(|| "—".into()),
                    last_gpu.map(|x| format!("{x}%")).unwrap_or_else(|| "—".into()),
                ),
            );
        }

        if let Ok(mut st) = estado.lock() {
            for (comp, msg) in disparos {
                logar(&mut st, format!("{hora} | ALERTA [{}]: {msg}", comp.nome()));
                st.alertas.push_back(Alerta { componente: comp, msg, hora: hora.clone() });
                while st.alertas.len() > 100 {
                    st.alertas.pop_front();
                }
            }
            st.inst = novo_estado.inst;
            st.cpu_media = novo_estado.cpu_media;
            st.ram_media = novo_estado.ram_media;
            st.temp_media = novo_estado.temp_media;
            st.ativos = novo_estado.ativos;
            st.log.extend(novo_estado.log);
            while st.log.len() > 300 {
                st.log.pop_front();
            }
        }
    }
}
