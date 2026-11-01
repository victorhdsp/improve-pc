#[derive(Default, Clone)]
pub struct Identificacao {
    pub board: String,
    pub bios: String,
    pub cpu: String,
    pub ram: String,
    pub gpu: String,
    pub disco: String,
    pub disco_uso: String,
    pub rede: Vec<(String, String)>,
    pub bateria: String,
}
