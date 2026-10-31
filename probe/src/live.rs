use std::thread;
use std::time::Duration;

use sysinfo::System;

pub fn report() -> (String, usize) {
    println!("=== CPU / RAM (ao vivo) ===");
    let mut sys = System::new();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    thread::sleep(Duration::from_millis(800));
    sys.refresh_cpu_usage();

    let cpus = sys.cpus();
    let (vendor_id, brand) = match cpus.first() {
        Some(c) => (c.vendor_id().to_string(), c.brand().trim().to_string()),
        None => ("desconhecido".into(), "desconhecido".into()),
    };
    println!(
        "CPU: {} | vendor: {} | núcleos lógicos: {} | uso: {:.1}%",
        brand,
        vendor_id,
        cpus.len(),
        sys.global_cpu_usage()
    );
    println!(
        "RAM: {:.2} / {:.2} GB",
        sys.used_memory() as f64 / 1_073_741_824.0,
        sys.total_memory() as f64 / 1_073_741_824.0
    );
    (vendor_id, cpus.len())
}
