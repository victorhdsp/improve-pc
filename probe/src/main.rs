mod board;
mod disks;
mod gpu;
mod live;
mod resident;
mod sensors;

#[cfg(windows)]
mod win;

fn main() {
    println!("# PROBE — o que dá pra ler nesta máquina\n");

    #[cfg(windows)]
    if msr_driver_rs::run_boot_service_if_requested() {
        return;
    }

    let install_driver = std::env::args().any(|a| a == "--install-driver");
    let loop_mode = std::env::args().any(|a| a == "--loop");

    #[cfg(windows)]
    println!(
        "elevação: {}",
        if win::admin::is_elevated() { "ADMIN" } else { "usuário comum" }
    );

    if loop_mode {
        resident::run(install_driver);
        return;
    }

    let (vendor_id, cores) = live::report();
    board::report();
    gpu::report();
    disks::report();
    sensors::report();

    #[cfg(windows)]
    {
        win::identify::report();
        win::identify::usage_report();
        win::msr_probe::report(&vendor_id, cores, install_driver);
    }

    #[cfg(not(windows))]
    {
        let _ = (vendor_id, cores, install_driver);
        println!("\n[Aviso: este binário rodou fora do alvo — identificação WMI e MSR/temps de CPU só existem no build Windows]");
    }
}
