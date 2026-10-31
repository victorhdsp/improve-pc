use ironmonitor::disk;

pub fn report() {
    println!("\n=== DISCOS ===");
    let devs = match disk::enumerate_disks() {
        Ok(d) => d,
        Err(e) => {
            println!("indisponível: {e:?}");
            return;
        }
    };
    for d in &devs {
        println!("--- {} ({:?}) ---", d.name(), d.disk_type());
        if let Ok(i) = d.info() {
            println!(
                "  modelo: {:?} | série: {:?} | capacidade: {:?} GB",
                i.model,
                i.serial,
                i.capacity.map(|c| c / 1_073_741_824)
            );
        }
        match d.smart_info() {
            Ok(s) => println!(
                "  SMART: temp {:?} °C | saúde {:?} | horas ligado {:?} | setores realocados {:?}",
                s.temperature, s.passed, s.power_on_hours, s.reallocated_sectors
            ),
            Err(e) => println!("  SMART: {e:?}"),
        }
    }
    if devs.is_empty() {
        println!("nenhum disco enumerado");
    }
}
