//! `minitel-tui` — télécommande du Minitel depuis le poste de travail.
//!
//! Se connecte à l'API de contrôle du daemon (`miniteld`, port 3010) et permet
//! d'écrire sur l'écran cathodique sans toucher au clavier d'époque : du texte
//! (paginé comme une réponse normale) ou une image (convertie en mosaïque G1
//! par `tools/img2vtx.py` si ce n'est pas déjà un `.vtx`).
//!
//! ```bash
//! cargo run --features tui --bin minitel-tui -- 192.168.1.42:3010
//! ```
//!
//! Dans le champ de saisie :
//!
//! | Entrée | Effet |
//! |---|---|
//! | du texte + Entrée | affiché sur le Minitel (`POST /text`) |
//! | `/img photo.png [--gray] [--row N]` | conversion `img2vtx.py` puis `POST /show` |
//! | `/vtx fichier.vtx` | envoi Vidéotex brut (`POST /show`) |
//! | `/help` | rappel des commandes |
//! | `/quit` ou Échap | quitter |
//!
//! L'état du lien série (connexion, bauds, veille, backend) est relu toutes les
//! 2 s dans la barre du haut — c'est le même JSON que `GET /status`.
//!
//! Ce binaire ne tourne **que sur le poste de travail** : il est derrière la
//! feature `tui` précisément pour que le build croisé du Pi ne le voie jamais.

use std::env;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

const DEFAULT_ADDR: &str = "127.0.0.1:3010";
/// Cadence de relecture de `GET /status` quand rien d'autre ne se passe.
const STATUS_EVERY: Duration = Duration::from_secs(2);

// ---------------------------------------------------------------------------
// HTTP minimal — même philosophie que le reste du dépôt : pas de dépendance
// HTTP pour trois routes en HTTP/1.0 (cf. AGENTS.md, conventions de code).
// ---------------------------------------------------------------------------

fn http(addr: &str, method: &str, path: &str, body: &[u8]) -> Result<Vec<u8>, String> {
    let sa = addr
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| format!("adresse invalide : {addr}"))?;
    let mut sock =
        TcpStream::connect_timeout(&sa, Duration::from_secs(3)).map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(Duration::from_secs(5))).ok();
    sock.set_write_timeout(Some(Duration::from_secs(5))).ok();
    let head = format!(
        "{method} {path} HTTP/1.0\r\nHost: {addr}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    sock.write_all(head.as_bytes())
        .and_then(|_| sock.write_all(body))
        .map_err(|e| e.to_string())?;
    let mut resp = Vec::new();
    sock.read_to_end(&mut resp).map_err(|e| e.to_string())?;
    let sep = resp
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("réponse tronquée")?;
    let status_line = resp[..sep]
        .split(|&b| b == b'\r')
        .next()
        .map(|l| String::from_utf8_lossy(l).to_string())
        .unwrap_or_default();
    if !status_line.contains(" 200 ") {
        return Err(status_line);
    }
    Ok(resp[sep + 4..].to_vec())
}

// ---------------------------------------------------------------------------
// Ouvrier réseau : tout le blocant (HTTP, conversion d'image) vit dans un
// thread pour que l'interface ne gèle jamais, même sur une IP injoignable.
// ---------------------------------------------------------------------------

enum Job {
    Text(String),
    /// Chemin d'une image + options passées telles quelles à `img2vtx.py`.
    Img(String, Vec<String>),
    Vtx(String),
}

enum Reply {
    Status(Result<DaemonStatus, String>),
    Info(String),
    Error(String),
}

#[derive(Clone, Default)]
struct DaemonStatus {
    connected: bool,
    baud: u64,
    sleeping: bool,
    net: String,
}

fn poll_status(addr: &str) -> Result<DaemonStatus, String> {
    let body = http(addr, "GET", "/status", b"")?;
    let v: serde_json::Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    Ok(DaemonStatus {
        connected: v["connected"].as_bool().unwrap_or(false),
        baud: v["baud"].as_u64().unwrap_or(0),
        sleeping: v["sleeping"].as_bool().unwrap_or(false),
        net: v["net"].as_str().unwrap_or("?").to_string(),
    })
}

/// Localise `tools/img2vtx.py` : variable d'env, dossier courant, ou à côté du
/// binaire (`target/<triple?>/release/minitel-tui` → racine du dépôt).
fn find_img2vtx() -> Option<PathBuf> {
    if let Ok(p) = env::var("MINITEL_IMG2VTX") {
        return Some(PathBuf::from(p));
    }
    let mut candidates = vec![PathBuf::from("tools/img2vtx.py")];
    if let Ok(exe) = env::current_exe() {
        let mut dir = exe.parent().map(PathBuf::from);
        while let Some(d) = dir {
            candidates.push(d.join("tools/img2vtx.py"));
            dir = d.parent().map(PathBuf::from);
        }
    }
    candidates.into_iter().find(|p| p.exists())
}

fn convert_image(path: &str, extra: &[String]) -> Result<Vec<u8>, String> {
    let script = find_img2vtx().ok_or(
        "tools/img2vtx.py introuvable — lancez depuis la racine du dépôt \
         ou pointez MINITEL_IMG2VTX vers le script",
    )?;
    let out = env::temp_dir().join("minitel-tui.vtx");
    let run = Command::new("python3")
        .arg(&script)
        .arg(path)
        .arg("-o")
        .arg(&out)
        .args(extra)
        .output()
        .map_err(|e| format!("python3 : {e}"))?;
    if !run.status.success() {
        return Err(String::from_utf8_lossy(&run.stderr).trim().to_string());
    }
    std::fs::read(&out).map_err(|e| e.to_string())
}

fn worker(addr: String, jobs: mpsc::Receiver<Job>, replies: mpsc::Sender<Reply>) {
    loop {
        // Entre deux commandes, on rafraîchit l'état ; après chaque commande
        // aussi (l'envoi d'un .vtx réveille le terminal, autant le montrer).
        let job = match jobs.recv_timeout(STATUS_EVERY) {
            Ok(j) => Some(j),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };
        if let Some(job) = job {
            let done = match job {
                Job::Text(t) => http(&addr, "POST", "/text", t.as_bytes())
                    .map(|_| format!("texte envoyé ({} car.)", t.chars().count())),
                Job::Img(path, extra) => convert_image(&path, &extra).and_then(|vtx| {
                    http(&addr, "POST", "/show", &vtx)
                        .map(|_| format!("{path} convertie et affichée ({} octets)", vtx.len()))
                }),
                Job::Vtx(path) => std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|vtx| {
                        http(&addr, "POST", "/show", &vtx)
                            .map(|_| format!("{path} affiché ({} octets)", vtx.len()))
                    }),
            };
            let _ = replies.send(match done {
                Ok(msg) => Reply::Info(msg),
                Err(e) => Reply::Error(e),
            });
        }
        let _ = replies.send(Reply::Status(poll_status(&addr)));
    }
}

// ---------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------

struct App {
    addr: String,
    status: Option<Result<DaemonStatus, String>>,
    /// Journal local : (couleur, texte). Le plus récent en dernier.
    history: Vec<(Color, String)>,
    input: String,
    /// Position du curseur dans `input`, en **caractères** (pas en octets).
    cursor: usize,
}

const HELP: &[&str] = &[
    "texte + Entrée        afficher sur le Minitel (paginé)",
    "/img photo.png        convertir puis afficher (options: --gray --row N)",
    "/vtx fichier.vtx      envoyer un flux Vidéotex brut",
    "/quit ou Échap        quitter",
];

impl App {
    fn push(&mut self, color: Color, msg: impl Into<String>) {
        self.history.push((color, msg.into()));
    }

    /// Interprète la ligne saisie. Retourne `false` pour quitter.
    fn submit(&mut self, jobs: &mpsc::Sender<Job>) -> bool {
        let line = std::mem::take(&mut self.input);
        self.cursor = 0;
        let line = line.trim().to_string();
        if line.is_empty() {
            return true;
        }
        match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["/quit" | "/q"] => return false,
            ["/help" | "/h"] => {
                for l in HELP {
                    self.push(Color::DarkGray, *l);
                }
            }
            ["/img", path, extra @ ..] => {
                self.push(Color::Yellow, format!("conversion de {path}…"));
                let _ = jobs.send(Job::Img(
                    path.to_string(),
                    extra.iter().map(|s| s.to_string()).collect(),
                ));
            }
            ["/vtx", path] => {
                let _ = jobs.send(Job::Vtx(path.to_string()));
            }
            [cmd, ..] if cmd.starts_with('/') => {
                self.push(Color::Red, format!("commande inconnue : {cmd} — /help"));
            }
            _ => {
                self.push(Color::Green, format!("> {line}"));
                let _ = jobs.send(Job::Text(line));
            }
        }
        true
    }

    fn status_line(&self) -> Line<'_> {
        let mut spans = vec![Span::styled(
            format!(" {} ", self.addr),
            Style::default().fg(Color::Cyan),
        )];
        match &self.status {
            None => spans.push(Span::styled("interrogation…", Style::default().fg(Color::DarkGray))),
            Some(Err(e)) => spans.push(Span::styled(
                format!("daemon injoignable ({e})"),
                Style::default().fg(Color::Red),
            )),
            Some(Ok(s)) => {
                let (label, color) = if s.connected {
                    (format!("Minitel OK {} bd", s.baud), Color::Green)
                } else {
                    ("Minitel déconnecté".to_string(), Color::Red)
                };
                spans.push(Span::styled(label, Style::default().fg(color)));
                if s.sleeping {
                    spans.push(Span::styled("  veille", Style::default().fg(Color::Yellow)));
                }
                let (net, color) = match s.net.as_str() {
                    "online" => ("backend OK", Color::Green),
                    "noweb" => ("backend sans Internet", Color::Yellow),
                    "offline" => ("backend KO", Color::Red),
                    _ => ("backend ?", Color::DarkGray),
                };
                spans.push(Span::raw("  ·  "));
                spans.push(Span::styled(net, Style::default().fg(color)));
            }
        }
        Line::from(spans)
    }
}

fn main() {
    let addr = env::args()
        .nth(1)
        .or_else(|| env::var("MINITEL_CTRL").ok())
        .unwrap_or_else(|| DEFAULT_ADDR.to_string());

    let (job_tx, job_rx) = mpsc::channel::<Job>();
    let (reply_tx, reply_rx) = mpsc::channel::<Reply>();
    {
        let addr = addr.clone();
        thread::spawn(move || worker(addr, job_rx, reply_tx));
    }

    let mut app = App {
        addr,
        status: None,
        history: Vec::new(),
        input: String::new(),
        cursor: 0,
    };
    app.push(Color::DarkGray, "minitel-tui — /help pour les commandes");

    let mut terminal = ratatui::init();
    loop {
        while let Ok(reply) = reply_rx.try_recv() {
            match reply {
                Reply::Status(s) => app.status = Some(s),
                Reply::Info(m) => app.push(Color::White, m),
                Reply::Error(e) => app.push(Color::Red, format!("échec : {e}")),
            }
        }

        terminal
            .draw(|f| {
                let [top, mid, bottom] = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .areas(f.area());

                f.render_widget(
                    Paragraph::new(app.status_line())
                        .block(Block::default().borders(Borders::ALL).title(" état ")),
                    top,
                );

                // Journal : on ne montre que ce qui tient, le plus récent en bas.
                let visible = mid.height.saturating_sub(2) as usize;
                let skip = app.history.len().saturating_sub(visible);
                let items: Vec<ListItem> = app.history[skip..]
                    .iter()
                    .map(|(c, m)| ListItem::new(Span::styled(m.clone(), Style::default().fg(*c))))
                    .collect();
                f.render_widget(
                    List::new(items)
                        .block(Block::default().borders(Borders::ALL).title(" journal ")),
                    mid,
                );

                // Rappel de la contrainte d'écran : 40 colonnes sur le Minitel.
                let count = app.input.chars().count();
                let title = format!(" saisie — {count} car. (lignes de 40 sur le Minitel) ");
                f.render_widget(
                    Paragraph::new(app.input.as_str())
                        .block(Block::default().borders(Borders::ALL).title(title)),
                    bottom,
                );
                f.set_cursor_position(Position::new(
                    bottom.x + 1 + app.cursor.min(bottom.width.saturating_sub(2) as usize) as u16,
                    bottom.y + 1,
                ));
            })
            .expect("dessin du terminal");

        if !event::poll(Duration::from_millis(120)).unwrap_or(false) {
            continue;
        }
        let Ok(Event::Key(key)) = event::read() else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Esc => break,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
            KeyCode::Enter => {
                if !app.submit(&job_tx) {
                    break;
                }
            }
            KeyCode::Backspace => {
                if app.cursor > 0 {
                    app.cursor -= 1;
                    let i = byte_index(&app.input, app.cursor);
                    app.input.remove(i);
                }
            }
            KeyCode::Delete => {
                if app.cursor < app.input.chars().count() {
                    let i = byte_index(&app.input, app.cursor);
                    app.input.remove(i);
                }
            }
            KeyCode::Left => app.cursor = app.cursor.saturating_sub(1),
            KeyCode::Right => app.cursor = (app.cursor + 1).min(app.input.chars().count()),
            KeyCode::Home => app.cursor = 0,
            KeyCode::End => app.cursor = app.input.chars().count(),
            KeyCode::Char(c) => {
                let i = byte_index(&app.input, app.cursor);
                app.input.insert(i, c);
                app.cursor += 1;
            }
            _ => {}
        }
    }
    ratatui::restore();
}

/// Index **octet** du n-ième caractère — `String::insert`/`remove` exigent une
/// frontière UTF-8, et la saisie est en français, donc accentuée.
fn byte_index(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map(|(i, _)| i).unwrap_or(s.len())
}
