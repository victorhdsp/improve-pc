mod board;
mod disks;
mod gpu;
mod live;
mod resident;
mod sensors;
mod update;

#[cfg(windows)]
mod win;

fn main() {
    #[cfg(windows)]
    if msr_driver_rs::run_boot_service_if_requested() {
        return;
    }

    let scan = std::env::args().any(|a| a == "--scan");
    let install_driver = std::env::args().any(|a| a == "--install-driver");
    let force_update = std::env::args().any(|a| a == "--update");

    #[cfg(windows)]
    println!(
        "elevação: {} | versão {}",
        if win::admin::is_elevated() { "ADMIN" } else { "usuário comum" },
        update::versao_atual()
    );

    if force_update {
        update::check(true);
        return;
    }

    if update::check(false) {
        println!("reiniciando com a nova versão...");
        self_update::restart::restart();
    }

    if scan {
        println!("# SCAN — o que dá pra ler nesta máquina\n");
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
        return;
    }

    resident::run();
}
