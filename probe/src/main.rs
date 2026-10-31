mod board;
mod disks;
mod gpu;
mod live;
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

    let (vendor_id, cores) = live::report();
    board::report();
    gpu::report();
    disks::report();
    sensors::report();

    #[cfg(windows)]
    {
        win::identify::report();
        win::msr_probe::report(&vendor_id, cores, install_driver);
    }

    #[cfg(not(windows))]
    {
        let _ = (vendor_id, cores, install_driver);
        println!("\n[Aviso: este binário rodou fora do alvo — identificação WMI e MSR/temps de CPU só existem no build Windows]");
    }
}
