# AGENTS.md — Monitor de PC (improve-pc)

App superleve de monitoramento de hardware em Rust para Windows: identifica os componentes do PC, monitora uso/temperatura em tempo real, detecta engasgos e se auto-atualiza. A interface mostra uma placa-mãe ilustrada em camadas (assets do usuário) com badges de alerta por componente.

## Estado atual (resumo)

- GUI (egui) com sistema de camadas por assets — **aguardando os assets finais** do usuário (`board_base.png`, `layer_cpu.png`, `layer_ram.png`, `layer_gpu.png`, `layer_m2.png`); sem eles, a GUI cai num fallback (blueprint) e continua funcional
- Modos: padrão = GUI; `--console` (terminal); `--scan` (diagnóstico one-shot); `--update` (força checagem); `--install-driver` (instala PawnIO, requer admin)
- Auto-update operante: tag `v*` → CI compila → GitHub Release → apps instalados atualizam sozinhos no próximo start

## Arquitetura

Crate único em `probe/`:

| Módulo | Papel |
|---|---|
| `monitor.rs` | Núcleo compartilhado: sampler 1 Hz, janela móvel 60s, regras de alerta com debounce, estado em `Arc<Mutex<MonitorState>>`. Alimenta console E GUI |
| `gui.rs` | Interface egui: carrega camadas PNG de `assets/` em runtime, hitbox automática (bounding box dos pixels opacos), ativação/hover/badges desenhados em código |
| `resident.rs` | Modo console: drena o log do estado compartilhado pro stdout |
| `win/msr.rs` | Temps de CPU por núcleo (MSR 0x1A2/0x19C) + consumo RAPL, via `msr-driver-rs` (PawnIO) |
| `win/identify.rs` | WMI: placa-mãe, BIOS, RAM chip a chip, rede (MSFT_NetworkAdapter), bateria, discos lógicos; monta `Identificacao` para a GUI. Roda em thread própria (ver Notas técnicas) |
| `update.rs` | `self_update` 1.3 (backend GitHub, ureq+rustls) |
| `ident.rs` | Struct `Identificacao` (dados estáticos exibidos na GUI) |

Deps e porquê: `sysinfo` (CPU/RAM ao vivo), `ironmonitor` (GPU multi-vendor, discos/SMART, board), `wmi` (identificação profunda), `msr-driver-rs` (temps nativas sem .NET/LHM), `self_update`, `eframe 0.31` (fixado de propósito, ver Notas técnicas), `image` (decode PNG), `chrono`.

CI (`.github/workflows/build.yml`): push em `main` = build + artifact; tag `v*` = build + Release (zip com `probe.exe` + PNGs de `assets/`).

Assets (`assets/`): o app procura a pasta ao lado do exe, depois `./assets`, depois `../assets`. Componente sem arquivo = não desenha (sem crash). GPU integrada (Intel Iris/UHD) não ganha layer — só GPU dedicada.

## Regras de trabalho

1. **Release exige aprovação explícita do usuário.** NUNCA bumpar versão, criar tag ou publicar release sem pedido/aprovação clara na conversa. Responder dúvida, analisar, discutir ou implementar código NÃO autoriza release — mesmo que o código esteja pronto e testado. Fluxo aprovado: bump em `probe/Cargo.toml` → commit → tag `vX.Y.Z` → push main + tag → conferir CI verde e Release publicada.

2. **Não inventar regras nem assumir nada.** Regras só entram nesta seção quando o usuário definir. Em caso de dúvida sobre como fazer algo, perguntar ao usuário em vez de decidir por conta própria.

## Notas técnicas (documentação factual, não regras)

- winit exige COM STA na thread main → qualquer WMI/COM (ex.: `resumo()`) roda em thread própria, senão `OleInitialize failed: RPC_E_CHANGED_MODE`
- egui `Rangef` usa `RangeInclusive` (`a..=b`, não `a..b`)
- ironmonitor: `DiskInfo.model` é `String` (não Option); `FirmwareType` não implementa Display (usar `{:?}`)
- WMI retorna campos numéricos (uint64/uint32) como Variant numérico — tipar como `u64`/`u32`, não `String`
- `eframe` fixado em 0.31 de propósito (0.37+ renomeou `App::update` → `App::ui` e reorganizou painéis)
- timeout em `cargo run` mata durante a compilação — buildar primeiro, rodar depois
- `Win32_PerfFormattedData_GPUEngine` não existe em todas as instalações (0x80041010) — uso de GPU vem do ironmonitor
- Verificação pré-push: `cargo check` (host) + `cargo check --target x86_64-pc-windows-gnu`; `ring`/`aws-lc-sys` não compilam no cross local (falta mingw) — o build Windows real é o CI, conferir com `gh run watch` até verde
- Assets: exportar do MESMO arquivo mestre (câmera/escala/canvas idênticos, fundo transparente, nomes `board_base`/`layer_cpu`/`layer_ram`/`layer_gpu`/`layer_m2`); nunca cromakear branco→alfa no código
- Dado indisponível deve ser reportado como indisponível (com motivo), não aproximado — princípio acordado na conversa

## Decisões de projeto registradas (contexto histórico)

- Rust single-binary, sem .NET/LHM/WebView — "superleve" é requisito
- Instalador (fase futura) instalará em `%LOCALAPPDATA%\Programs` para o auto-update continuar sem elevação; o driver PawnIO é instalado pelo app na primeira execução como admin
- Auto-update troca o exe em `current_exe()` e reinicia; assets via runtime folder (usuário itera arte sem release)
- Seleção/hover/ativação são desenhados em código por cima da layer (não existe versão "ativada" de asset)

## Licenças (atenção para o futuro)

`ironmonitor` = AGPL-3.0, `msr-driver-rs` = GPL-3.0. OK para uso pessoal/open-source; impedem fechar o código no formato atual. Se um dia for comercializar fechado, será preciso substituir essas duas dependências.
