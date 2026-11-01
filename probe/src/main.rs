mod board;
mod disks;
mod gpu;
mod gui;
mod ident;
mod live;
mod monitor;
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
    let console = std::env::args().any(|a| a == "--console");
    let install_driver = std::env::args().any(|a| a == "--install-driver");

    #[cfg(windows)]
    println!(
        "elevação: {} | versão {}",
        if win::admin::is_elevated() { "ADMIN" } else { "usuário comum" },
        update::versao_atual()
    );

    if std::env::args().any(|a| a == "--update") {
        update::check(true);
        return;
    }

    if update::check(false) {
        println!("reiniciando com a nova versão...");
        let _ = self_update::restart::restart();
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

    if console {
        resident::run();
        return;
    }

    // WMI inicializa COM em modo MTA na thread chamadora; o winit exige STA na main.
    // Isolamos a identificação numa thread própria pra não contaminar a thread principal.
    #[cfg(windows)]
    let ident = std::thread::spawn(win::identify::resumo)
        .join()
        .unwrap_or_default();
    #[cfg(not(windows))]
    let ident = ident::Identificacao::default();

    let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gui::run(ident)));
    match resultado {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("falha ao abrir a janela: {e}");
        }
        Err(_) => {
            eprintln!("erro interno inesperado ao abrir a janela (panic)");
        }
    }
    eprintln!("caí para o modo console — Ctrl+C para sair");
    resident::run();
}
