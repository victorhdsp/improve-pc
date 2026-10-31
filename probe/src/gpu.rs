use ironmonitor::gpu::GpuCollection;

pub fn report() {
    println!("\n=== GPUs ===");
    let gpus = match GpuCollection::auto_detect() {
        Ok(g) => g,
        Err(e) => {
            println!("indisponível: {e:?}");
            println!("  (o nome do modelo aparece na seção WMI abaixo)");
            return;
        }
    };
    match gpus.snapshot_all() {
        Ok(list) => {
            if list.is_empty() {
                println!("nenhuma GPU detectada");
            }
            for g in &list {
                let s = &g.static_info;
                let d = &g.dynamic_info;
                println!("--- {} [{:?}] ---", s.name, s.vendor);
                println!("  driver: {:?}", s.driver_version);
                println!("  uso: {:?}", d.utilization);
                println!("  temp: {:?} °C", d.thermal.temperature);
                println!("  fan: {:?} rpm", d.thermal.fan_rpm);
                println!("  memória usada: {:?}", d.memory.used);
            }
        }
        Err(e) => println!("snapshot falhou: {e:?}"),
    }
}
