use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke};

use crate::ident::Identificacao;
use crate::monitor::{self, Componente, MonitorState, Shared};
use crate::update::{self, ResultadoUpdate};

const FUNDO: Color32 = Color32::from_rgb(22, 24, 30);
const CARTAO: Color32 = Color32::from_rgb(244, 245, 248);
const SEL: Color32 = Color32::from_rgb(240, 200, 60);
const ALERTA: Color32 = Color32::from_rgb(232, 72, 60);
const USO: Color32 = Color32::from_rgb(70, 140, 210);
const TEXTO: Color32 = Color32::from_rgb(222, 224, 230);
const TEXTO_FRACO: Color32 = Color32::from_rgb(150, 154, 164);

const CAMADAS: &[(&str, Componente)] = &[
    ("board_base.png", Componente::Placa),
    ("layer_cpu.png", Componente::Cpu),
    ("layer_ram.png", Componente::Ram),
    ("layer_gpu.png", Componente::Gpu),
    ("layer_m2.png", Componente::Disco),
];

fn pasta_assets() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            dirs.push(p.join("assets"));
        }
    }
    dirs.push(std::path::PathBuf::from("assets"));
    dirs.push(std::path::PathBuf::from("../assets"));
    dirs
}

struct Camada {
    tex: egui::TextureHandle,
    bbox: Rect,
    componente: Componente,
}

fn carregar_camadas(ctx: &egui::Context) -> Vec<Camada> {
    let dirs = pasta_assets();
    let mut out = Vec::new();
    for (arquivo, componente) in CAMADAS {
        let Some(caminho) = dirs.iter().map(|d| d.join(arquivo)).find(|c| c.exists()) else {
            continue;
        };
        let Ok(bytes) = std::fs::read(&caminho) else { continue };
        let Ok(img) = image::load_from_memory(&bytes) else { continue };
        let img = img.to_rgba8();
        let (w, h) = (img.width() as f32, img.height() as f32);
        let mut bbox: Option<(u32, u32, u32, u32)> = None;
        for (x, y, px) in img.enumerate_pixels() {
            if px.0[3] > 16 {
                bbox = Some(match bbox {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            }
        }
        let size = [img.width() as usize, img.height() as usize];
        let tex = ctx.load_texture(
            format!("camada_{arquivo}"),
            egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        let bbox = match bbox {
            Some((x0, y0, x1, y1)) if *componente != Componente::Placa => Rect::from_min_max(
                Pos2::new(x0 as f32 / w, y0 as f32 / h),
                Pos2::new((x1 + 1) as f32 / w, (y1 + 1) as f32 / h),
            ),
            _ => Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        };
        out.push(Camada { tex, bbox, componente: *componente });
    }
    out
}

fn gpu_e_integrada(nome: &str) -> bool {
    let n = nome.to_uppercase();
    n.contains("IRIS") || n.contains("UHD") || n.contains("GRAPHICS")
}

struct Hotspot {
    componente: Componente,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

const HOTSPOTS: &[Hotspot] = &[
    Hotspot { componente: Componente::Cpu, x0: 0.31, y0: 0.40, x1: 0.53, y1: 0.66 },
    Hotspot { componente: Componente::Ram, x0: 0.16, y0: 0.24, x1: 0.52, y1: 0.52 },
    Hotspot { componente: Componente::Gpu, x0: 0.55, y0: 0.32, x1: 0.84, y1: 0.56 },
    Hotspot { componente: Componente::Disco, x0: 0.40, y0: 0.60, x1: 0.58, y1: 0.80 },
    Hotspot { componente: Componente::Bateria, x0: 0.62, y0: 0.35, x1: 0.73, y1: 0.49 },
    Hotspot { componente: Componente::Rede, x0: 0.64, y0: 0.69, x1: 0.71, y1: 0.80 },
];

pub fn run(ident: Identificacao) -> Result<(), eframe::Error> {
    let estado: Shared = Arc::new(Mutex::new(MonitorState::default()));
    monitor::iniciar(estado.clone());
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1060.0, 720.0])
            .with_min_inner_size([820.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Monitor de PC",
        opts,
        Box::new(move |cc| Ok(Box::new(App::new(&cc.egui_ctx, estado.clone(), ident)))),
    )
}

struct App {
    estado: Shared,
    ident: Identificacao,
    selecionado: Option<Componente>,
    console_aberto: bool,
    upd_status: Arc<Mutex<Option<String>>>,
    upd_rodando: Arc<Mutex<bool>>,
    reiniciar: bool,
    camadas: Vec<Camada>,
}

impl App {
    fn new(ctx: &egui::Context, estado: Shared, ident: Identificacao) -> Self {
        let camadas = carregar_camadas(ctx);
        Self {
            estado,
            ident,
            selecionado: Some(Componente::Cpu),
            console_aberto: false,
            upd_status: Arc::new(Mutex::new(None)),
            upd_rodando: Arc::new(Mutex::new(false)),
            reiniciar: false,
            camadas,
        }
    }
}

fn uso_do(estado: &MonitorState, c: Componente) -> Option<f32> {
    match c {
        Componente::Cpu => Some(estado.inst.as_ref()?.cpu),
        Componente::Ram => Some(estado.inst.as_ref()?.ram),
        Componente::Gpu => estado.inst.as_ref()?.gpu.map(|g| g as f32),
        Componente::Disco => None,
        _ => None,
    }
}

impl App {
    fn detalhes(&self, st: &MonitorState, c: Componente) -> Vec<(String, String)> {
        let inst = st.inst.clone();
        match c {
            Componente::Cpu => vec![
                ("modelo".into(), self.ident.cpu.clone()),
                ("uso agora".into(), inst.as_ref().map(|i| format!("{:.0}%", i.cpu)).unwrap_or_default()),
                ("uso médio 60s".into(), format!("{:.0}%", st.cpu_media)),
                ("temp máx".into(), inst.as_ref().and_then(|i| i.temp).map(|t| format!("{t:.0} °C")).unwrap_or_else(|| "—".into())),
                ("temp média 60s".into(), st.temp_media.map(|t| format!("{t:.0} °C")).unwrap_or_else(|| "—".into())),
                ("consumo package".into(), inst.as_ref().and_then(|i| i.pkg_w).map(|w| format!("{w:.1} W")).unwrap_or_else(|| "—".into())),
            ],
            Componente::Ram => vec![
                ("total".into(), inst.as_ref().map(|i| format!("{:.1} GB", i.ram_total_gb)).unwrap_or_default()),
                ("em uso".into(), inst.as_ref().map(|i| format!("{:.1} GB ({:.0}%)", i.ram_usada_gb, i.ram)).unwrap_or_default()),
                ("média 60s".into(), format!("{:.0}%", st.ram_media)),
                ("montagem".into(), self.ident.ram.clone()),
            ],
            Componente::Gpu => vec![
                ("modelo".into(), self.ident.gpu.clone()),
                ("uso".into(), inst.as_ref().and_then(|i| i.gpu).map(|g| format!("{g}%")).unwrap_or_else(|| "—".into())),
                ("memória em uso".into(), inst.as_ref().and_then(|i| i.gpu_mem_gb).map(|m| format!("{m:.1} GB")).unwrap_or_else(|| "—".into())),
            ],
            Componente::Disco => vec![
                ("modelo".into(), self.ident.disco.clone()),
                ("partições".into(), self.ident.disco_uso.clone()),
            ],
            Componente::Placa => vec![
                ("placa".into(), self.ident.board.clone()),
                ("BIOS".into(), self.ident.bios.clone()),
            ],
            Componente::Rede => self
                .ident
                .rede
                .iter()
                .map(|(n, d)| (n.clone(), d.clone()))
                .collect(),
            Componente::Bateria => vec![("estado".into(), self.ident.bateria.clone())],
        }
    }

    fn painel_topo(&mut self, ui: &mut egui::Ui, st: &MonitorState) {
        ui.horizontal(|ui| {
            if ui.button("☰").clicked() {
                self.console_aberto = !self.console_aberto;
            }
            ui.label(egui::RichText::new("Monitor de PC").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let rodando = *self.upd_rodando.lock().unwrap();
                let botao = egui::Button::new(if rodando { "verificando…" } else { "Atualizar agora" });
                if ui.add_enabled(!rodando, botao).clicked() {
                    *self.upd_rodando.lock().unwrap() = true;
                    let status = self.upd_status.clone();
                    let rodando_flag = self.upd_rodando.clone();
                    thread::spawn(move || {
                        let r = update::verificar();
                        *status.lock().unwrap() = Some(match r {
                            Ok(ResultadoUpdate::EmDia(v)) => format!("versão {v}: em dia"),
                            Ok(ResultadoUpdate::Atualizado(v)) => format!("atualizado para {v} — reinicie"),
                            Err(e) => format!("falhou: {e}"),
                        });
                        *rodando_flag.lock().unwrap() = false;
                    });
                }
                if let Some(s) = self.upd_status.lock().unwrap().clone() {
                    ui.label(egui::RichText::new(s).small());
                }
            });
        });
        ui.separator();

        match self.selecionado {
            Some(c) => {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(c.nome()).strong().size(16.0));
                    if st.ativos.contains(&c) {
                        ui.label(egui::RichText::new("⚠ ATENÇÃO").color(ALERTA).strong());
                    }
                });
                egui::Grid::new("detalhes").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
                    for (k, v) in self.detalhes(st, c) {
                        ui.label(egui::RichText::new(k).color(TEXTO_FRACO).small());
                        ui.label(egui::RichText::new(v).small());
                        ui.end_row();
                    }
                });
                let relacionados: Vec<_> = st.alertas.iter().rev().filter(|a| a.componente == c).take(4).collect();
                for a in relacionados {
                    ui.label(
                        egui::RichText::new(format!("{} — {}", a.hora, a.msg))
                            .small()
                            .color(ALERTA),
                    );
                }
            }
            None => {
                ui.label(egui::RichText::new("clique em um componente da placa para ver os dados").color(TEXTO_FRACO));
            }
        }
    }

    fn desenhar_placa(&mut self, ui: &mut egui::Ui, st: &MonitorState) {
        let (area, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let painter = ui.painter_at(area);
        painter.rect_filled(area, 0.0, FUNDO);

        if self.camadas.is_empty() {
            self.desenhar_fallback(ui, area, st);
            return;
        }

        let base = &self.camadas[0];
        let ts = base.tex.size_vec2();
        let escala = (area.width() / ts.x).min(area.height() / ts.y) * 0.96;
        let img_rect = Rect::from_center_size(area.center(), ts * escala);
        painter.rect_filled(img_rect.expand(6.0), 10.0, CARTAO);

        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let tempo = ui.input(|i| i.time) as f32;

        for (i, cam) in self.camadas.iter().enumerate() {
            if i > 0 && !self.camada_presente(cam.componente) {
                continue;
            }
            painter.image(cam.tex.id(), img_rect, uv, Color32::WHITE);

            let r = Rect::from_min_max(
                Pos2::new(img_rect.left() + img_rect.width() * cam.bbox.left(), img_rect.top() + img_rect.height() * cam.bbox.top()),
                Pos2::new(img_rect.left() + img_rect.width() * cam.bbox.right(), img_rect.top() + img_rect.height() * cam.bbox.bottom()),
            );
            let selecionado = self.selecionado == Some(cam.componente);
            let resp = ui.interact(r, egui::Id::new(format!("h_{:?}", cam.componente)), Sense::click());
            if resp.clicked() {
                self.selecionado = Some(cam.componente);
            }

            if let Some(u) = uso_do(st, cam.componente) {
                let altura = r.height() * (u / 100.0).clamp(0.03, 1.0);
                painter.rect_filled(
                    Rect::from_min_max(Pos2::new(r.left(), r.bottom() - altura), Pos2::new(r.right(), r.bottom())),
                    3.0,
                    USO.gamma_multiply(0.60),
                );
            }

            if selecionado {
                painter.rect_filled(r, 6.0, SEL.gamma_multiply(0.18));
                painter.rect_stroke(r, 6.0, Stroke::new(2.5_f32, SEL), egui::StrokeKind::Inside);
            } else if resp.hovered() {
                painter.rect_filled(r, 6.0, Color32::from_rgba_unmultiplied(255, 255, 255, 26));
                painter.rect_stroke(r, 6.0, Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 170)), egui::StrokeKind::Inside);
            }

            if st.ativos.contains(&cam.componente) {
                let centro = Pos2::new(r.right(), r.top());
                let raio = 10.0 + 2.5 * (tempo * 5.0).sin();
                painter.circle_filled(centro, raio, ALERTA);
                painter.text(centro, Align2::CENTER_CENTER, "!", FontId::proportional(15.0), Color32::WHITE);
            }
        }

        let fundo = ui.interact(img_rect, egui::Id::new("fundo_placa"), Sense::click());
        if fundo.clicked() {
            self.selecionado = Some(Componente::Placa);
        }

        painter.text(
            Pos2::new(area.left() + 12.0, area.bottom() - 10.0),
            Align2::LEFT_BOTTOM,
            &self.ident.board,
            FontId::proportional(11.0),
            TEXTO_FRACO,
        );
    }

    fn camada_presente(&self, c: Componente) -> bool {
        match c {
            Componente::Cpu | Componente::Ram | Componente::Disco => true,
            Componente::Gpu => !(self.ident.gpu.is_empty() || gpu_e_integrada(&self.ident.gpu)),
            _ => false,
        }
    }

    fn desenhar_fallback(&mut self, ui: &mut egui::Ui, area: Rect, st: &MonitorState) {
        let painter = ui.painter_at(area);
        painter.text(
            area.center(),
            Align2::CENTER_CENTER,
            "coloque os PNGs das camadas na pasta assets/ ao lado do probe.exe",
            FontId::proportional(13.0),
            TEXTO_FRACO,
        );
        let tempo = ui.input(|i| i.time) as f32;
        for h in HOTSPOTS {
            let r = Rect::from_min_max(
                Pos2::new(area.left() + area.width() * h.x0, area.top() + area.height() * h.y0),
                Pos2::new(area.left() + area.width() * h.x1, area.top() + area.height() * h.y1),
            );
            let selecionado = self.selecionado == Some(h.componente);
            if selecionado {
                painter.rect_stroke(r, 6.0, Stroke::new(2.5_f32, SEL), egui::StrokeKind::Inside);
            }
            let resp = ui.interact(r, egui::Id::new(format!("f_{:?}", h.componente)), Sense::click());
            if resp.clicked() {
                self.selecionado = Some(h.componente);
            }
            if st.ativos.contains(&h.componente) {
                let centro = Pos2::new(r.right(), r.top());
                let raio = 10.0 + 2.5 * (tempo * 5.0).sin();
                painter.circle_filled(centro, raio, ALERTA);
                painter.text(centro, Align2::CENTER_CENTER, "!", FontId::proportional(15.0), Color32::WHITE);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let st = self.estado.lock().unwrap().clone();

        egui::TopBottomPanel::top("topo").show(ctx, |ui| self.painel_topo(ui, &st));

        egui::TopBottomPanel::bottom("rodape").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(i) = &st.inst {
                    ui.label(format!(
                        "cpu {:.0}%  |  ram {:.0}%  |  temp {}  |  pkg {}  |  gpu {}",
                        i.cpu,
                        i.ram,
                        i.temp.map(|t| format!("{t:.0}°C")).unwrap_or_else(|| "—".into()),
                        i.pkg_w.map(|w| format!("{w:.1}W")).unwrap_or_else(|| "—".into()),
                        i.gpu.map(|g| format!("{g}%")).unwrap_or_else(|| "—".into()),
                    ));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("console").clicked() {
                        self.console_aberto = !self.console_aberto;
                    }
                });
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(FUNDO))
            .show(ctx, |ui| self.desenhar_placa(ui, &st));

        if self.console_aberto {
            egui::Window::new("Console do sistema")
                .default_width(700.0)
                .default_height(220.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                        for linha in &st.log {
                            ui.label(egui::RichText::new(linha).monospace().small().color(TEXTO));
                        }
                    });
                });
        }

        if self.reiniciar {
            let _ = self_update::restart::restart();
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}
