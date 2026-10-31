use ironmonitor::hwmon::HardwareMonitor;

pub fn report() {
    println!("\n=== SENSORES (hwmon) ===");
    let mut hm = HardwareMonitor::new();
    hm.refresh();
    if hm.sensors.is_empty() {
        println!("nenhum sensor legível (no Windows sem driver/monitor instalado: esperado)");
        return;
    }
    for s in &hm.sensors {
        println!(
            "[{:?}/{:?}] {} = {:.1}",
            s.hardware_type, s.sensor_type, s.name, s.value
        );
    }
}
