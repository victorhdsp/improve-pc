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
| `win/identify.rs` | WMI: placa-mãe, BIOS, RAM chip a chip, rede (MSFT_NetworkAdapter), bateria, discos lógicos; monta `Identificacao` para a GUI. **Roda em thread própria** (ver regra 5) |
| `update.rs` | `self_update` 1.3 (backend GitHub, ureq+rustls) |
| `ident.rs` | Struct `Identificacao` (dados estáticos exibidos na GUI) |

Deps e porquê: `sysinfo` (CPU/RAM ao vivo), `ironmonitor` (GPU multi-vendor, discos/SMART, board), `wmi` (identificação profunda), `msr-driver-rs` (temps nativas sem .NET/LHM), `self_update`, `eframe 0.31` (fixado de propósito), `image` (decode PNG), `chrono`.

CI (`.github/workflows/build.yml`): push em `main` = build + artifact; tag `v*` = build + Release (zip com `probe.exe` + PNGs de `assets/`).

Assets (`assets/`): camadas transparentes exportadas do MESMO mestre (mesma câmera/escala/canvas). O app procura a pasta `assets/` ao lado do exe, depois `./assets`, depois `../assets`. Componente sem arquivo = não desenha (sem crash). GPU integrada (Intel Iris/UHD) não ganha layer — só GPU dedicada.

## Regras de trabalho

1. **Release exige aprovação explícita do usuário.** NUNCA bumpar versão, criar tag ou publicar release sem pedido/aprovação clara na conversa. Responder dúvida, analisar, discutir ou implementar código NÃO autoriza release — mesmo que o código esteja pronto e testado. Fluxo aprovado: bump em `probe/Cargo.toml` → commit → tag `vX.Y.Z` → push main + tag → conferir CI verde e Release publicada.

2. **Dado honesto ou nenhum dado.** Se um sensor/valor não está disponível, reportar indisponível com motivo — nunca inventar número nem aproximação. Erros de leitura ficam visíveis, não silenciados.

3. **Verificação antes de qualquer push**: `cargo check` (host) + `cargo check --target x86_64-pc-windows-gnu`. Limitação conhecida: `ring`/`aws-lc-sys` não compilam no cross local (falta mingw com sudo) — isso é esperado; o build Windows real é o CI, sempre conferir com `gh run watch` até verde antes de considerar pronto.

4. **Idioma da UI e mensagens: português.** Commits em português, sem emojis.

5. **Armadilhas conhecidas (não recair):**
   - winit exige COM STA na thread main → qualquer WMI/COM (ex.: `resumo()`) DEVE rodar em thread própria, senão `OleInitialize failed: RPC_E_CHANGED_MODE`
   - egui `Rangef` usa `RangeInclusive` (`a..=b`, não `a..b`)
   - ironmonitor: `DiskInfo.model` é `String` (não Option); `FirmwareType` não implementa Display (usar `{:?}`)
   - WMI retorna campos numéricos (uint64/uint32) como Variant numérico — tipar como `u64`/`u32`, não `String`
   - `eframe` fixado em 0.31 de propósito (0.37+ renomeou `App::update` → `App::ui` e reorganizou painéis)
   - timeout em `cargo run` mata durante a compilação — buildar primeiro, rodar depois
   - `Win32_PerfFormattedData_GPUEngine` não existe em todas as instalações (0x80041010) — uso de GPU vem do ironmonitor

6. **Assets**: só do mesmo arquivo mestre, câmera/escala/canvas idênticos, fundo transparente, nomes exatos (`board_base`, `layer_cpu`, `layer_ram`, `layer_gpu`, `layer_m2`). Nunca cromakear branco→alfa no código (componentes brancos seriam destruídos).

## Decisões de projeto (não reabrir sem o usuário)

- Rust single-binary, sem .NET/LHM/WebView — "superleve" é requisito
- Instalador (fase futura) instalará em `%LOCALAPPDATA%\Programs` para o auto-update continuar sem elevação; o driver PawnIO é instalado pelo app na primeira execução como admin
- Auto-update troca o exe em `current_exe()` e reinicia; assets via runtime folder (usuário itera arte sem release)
- Versão "ativada" dos assets NÃO existe: seleção/hover são desenhados em código por cima da layer

## Licenças (atenção para o futuro)

`ironmonitor` = AGPL-3.0, `msr-driver-rs` = GPL-3.0. OK para uso pessoal/open-source; **impedem** fechar o código no formato atual. Se um dia for comercializar fechado, será preciso substituir essas duas dependências.
