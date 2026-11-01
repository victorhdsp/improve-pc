use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke};

use crate::ident::Identificacao;
use crate::monitor::{self, Componente, MonitorState, Shared};
use crate::update::{self, ResultadoUpdate};

const FUNDO: Color32 = Color32::from_rgb(22, 24, 30);
const PLACA: Color32 = Color32::from_rgb(28, 52, 40);
const PLACA_BORDA: Color32 = Color32::from_rgb(58, 105, 78);
const TRILHA: Color32 = Color32::from_rgb(46, 84, 62);
const PECA: Color32 = Color32::from_rgb(50, 54, 64);
const PECA_HOVER: Color32 = Color32::from_rgb(66, 71, 84);
const PECA_BORDA: Color32 = Color32::from_rgb(108, 114, 128);
const SEL: Color32 = Color32::from_rgb(240, 200, 60);
const ALERTA: Color32 = Color32::from_rgb(232, 72, 60);
const USO: Color32 = Color32::from_rgb(90, 160, 220);
const TEXTO: Color32 = Color32::from_rgb(222, 224, 230);
const TEXTO_FRACO: Color32 = Color32::from_rgb(150, 154, 164);

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
        Box::new(move |_cc| Ok(Box::new(App::new(estado.clone(), ident)))),
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
}

impl App {
    fn new(estado: Shared, ident: Identificacao) -> Self {
        Self {
            estado,
            ident,
            selecionado: Some(Componente::Cpu),
            console_aberto: false,
            upd_status: Arc::new(Mutex::new(None)),
            upd_rodando: Arc::new(Mutex::new(false)),
            reiniciar: false,
        }
    }
}

struct Widget {
    componente: Componente,
    rotulo: &'static str,
    rect: Rect,
}

fn layout(total: Rect) -> (Rect, Vec<Widget>) {
    let nx = |a: f32, b: f32| total.min.x + total.width() * a..=total.min.x + total.width() * b;
    let ny = |a: f32, b: f32| total.min.y + total.height() * a..=total.min.y + total.height() * b;
    let r = |x: (RangeInclusive<f32>, RangeInclusive<f32>)| Rect::from_x_y_ranges(x.0, x.1);

    let placa = r((nx(0.06, 0.94), ny(0.08, 0.94)));
    let widgets = vec![
        Widget { componente: Componente::Cpu, rotulo: "CPU", rect: r((nx(0.38, 0.58), ny(0.14, 0.36))) },
        Widget { componente: Componente::Ram, rotulo: "RAM A", rect: r((nx(0.64, 0.69), ny(0.12, 0.38))) },
        Widget { componente: Componente::Ram, rotulo: "RAM B", rect: r((nx(0.72, 0.77), ny(0.12, 0.38))) },
        Widget { componente: Componente::Gpu, rotulo: "GPU", rect: r((nx(0.20, 0.58), ny(0.50, 0.64))) },
        Widget { componente: Componente::Disco, rotulo: "M.2", rect: r((nx(0.34, 0.58), ny(0.72, 0.80))) },
        Widget { componente: Componente::Rede, rotulo: "RED", rect: r((nx(0.12, 0.19), ny(0.66, 0.74))) },
        Widget { componente: Componente::Bateria, rotulo: "BAT", rect: r((nx(0.12, 0.24), ny(0.80, 0.88))) },
    ];
    (placa, widgets)
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

        let (placa, widgets) = layout(area);
        painter.rect_filled(placa, 10.0, PLACA);
        painter.rect_stroke(placa, 10.0, Stroke::new(1.5_f32, PLACA_BORDA), egui::StrokeKind::Inside);

        for trilha in [
            Rect::from_x_y_ranges(placa.left() + 30.0..=placa.right() - 30.0, placa.top() + 8.0..=placa.top() + 10.0),
            Rect::from_x_y_ranges(placa.left() + 30.0..=placa.right() - 30.0, placa.bottom() - 10.0..=placa.bottom() - 8.0),
        ] {
            painter.rect_filled(trilha, 1.0, TRILHA);
        }

        let tempo = ui.input(|i| i.time) as f32;
        for w in &widgets {
            let hover = ui
                .interact(w.rect, egui::Id::new(format!("w_{:?}", w.componente)), Sense::click())
                .hovered();
            let selecionado = self.selecionado == Some(w.componente);
            let borda = if selecionado {
                Stroke::new(2.0_f32, SEL)
            } else {
                Stroke::new(1.2_f32, PECA_BORDA)
            };
            let fill = if hover { PECA_HOVER } else { PECA };

            painter.rect_filled(w.rect, 5.0, fill);
            painter.rect_stroke(w.rect, 5.0, borda, egui::StrokeKind::Inside);

            if let Some(u) = uso_do(st, w.componente) {
                let altura = w.rect.height() * (u / 100.0).clamp(0.02, 1.0);
                let faixa = Rect::from_x_y_ranges(
                    w.rect.left() + 2.0..=w.rect.right() - 2.0,
                    w.rect.bottom() - 2.0 - altura..=w.rect.bottom() - 2.0,
                );
                painter.rect_filled(faixa, 2.0, USO.gamma_multiply(0.55));
            }

            painter.text(
                Pos2::new(w.rect.center().x, w.rect.top() + 12.0),
                Align2::CENTER_TOP,
                w.rotulo,
                FontId::proportional(11.0),
                TEXTO,
            );

            if st.ativos.contains(&w.componente) {
                let centro = Pos2::new(w.rect.right() - 4.0, w.rect.top() + 4.0);
                let raio = 9.0 + 2.0 * (tempo * 5.0).sin();
                painter.circle_filled(centro, raio, ALERTA);
                painter.text(
                    centro,
                    Align2::CENTER_CENTER,
                    "!",
                    FontId::proportional(14.0),
                    Color32::WHITE,
                );
            }

            let resp = ui.interact(w.rect, egui::Id::new(format!("c_{:?}", w.componente)), Sense::click());
            if resp.clicked() {
                self.selecionado = Some(w.componente);
            }
        }

        painter.text(
            Pos2::new(placa.right() - 10.0, placa.bottom() - 10.0),
            Align2::RIGHT_BOTTOM,
            &self.ident.board,
            FontId::proportional(11.0),
            TEXTO_FRACO,
        );
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
