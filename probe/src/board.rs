use ironmonitor::motherboard;

pub fn report() {
    println!("\n=== PLACA-MÃE / BIOS / OS ===");
    match motherboard::get_system_info() {
        Ok(info) => {
            println!("board: {:?} / {:?}", info.board_vendor, info.board_name);
            println!("sistema: {:?} / {:?}", info.manufacturer, info.product_name);
            println!(
                "BIOS: {:?} {:?} (rev {:?}, firmware {:?})",
                info.bios.vendor, info.bios.version, info.bios.revision, info.bios.firmware_type
            );
            println!("Secure Boot: {:?}", info.bios.secure_boot);
            println!("OS: {:?} {:?} / {:?}", info.os_name, info.os_version, info.architecture);
            println!(
                "hostname: {:?} | serial: {:?} | uuid: {:?}",
                info.hostname, info.serial_number, info.uuid
            );
        }
        Err(e) => println!("indisponível: {e:?}"),
    }
}
