use self_update::cargo_crate_version;

const REPO_OWNER: &str = "victorhdsp";
const REPO_NAME: &str = "improve-pc";

pub fn versao_atual() -> &'static str {
    cargo_crate_version!()
}

pub fn check(verbose: bool) -> bool {
    let result = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("probe")
        .bin_path_in_archive("probe.exe")
        .current_version(cargo_crate_version!())
        .show_output(verbose)
        .no_confirm(true)
        .build()
        .and_then(|u| u.update());

    match result {
        Ok(status) => {
            let v = status.version();
            match status {
                self_update::VersionStatus::Updated(_) => {
                    println!("atualizado para {v}");
                    true
                }
                _ => {
                    if verbose {
                        println!("versão {v}: em dia");
                    }
                    false
                }
            }
        }
        Err(e) => {
            if verbose {
                println!("checagem de atualização falhou: {e}");
            }
            false
        }
    }
}
