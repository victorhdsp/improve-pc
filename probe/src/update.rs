use self_update::cargo_crate_version;

const REPO_OWNER: &str = "victorhdsp";
const REPO_NAME: &str = "improve-pc";

pub enum ResultadoUpdate {
    EmDia(String),
    Atualizado(String),
}

#[allow(dead_code)]
pub fn versao_atual() -> &'static str {
    cargo_crate_version!()
}

pub fn verificar() -> Result<ResultadoUpdate, String> {
    let result = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("probe")
        .bin_path_in_archive("probe.exe")
        .current_version(cargo_crate_version!())
        .show_output(false)
        .no_confirm(true)
        .build()
        .and_then(|u| u.update());

    match result {
        Ok(status) => {
            let v = status.version().to_string();
            if matches!(status, self_update::VersionStatus::Updated(_)) {
                Ok(ResultadoUpdate::Atualizado(v))
            } else {
                Ok(ResultadoUpdate::EmDia(v))
            }
        }
        Err(e) => Err(format!("{e}")),
    }
}

pub fn check(verbose: bool) -> bool {
    match verificar() {
        Ok(ResultadoUpdate::Atualizado(v)) => {
            println!("atualizado para {v}");
            true
        }
        Ok(ResultadoUpdate::EmDia(v)) => {
            if verbose {
                println!("versão {v}: em dia");
            }
            false
        }
        Err(e) => {
            if verbose {
                println!("checagem de atualização falhou: {e}");
            }
            false
        }
    }
}
