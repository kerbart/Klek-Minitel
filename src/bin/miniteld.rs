//! `miniteld` — daemon Minitel : un terminal de conversation sur du vrai
//! matériel Vidéotex.
//!
//! L'utilisateur tape une requête, **Envoi** la lance. Le daemon la transmet au
//! backend HTTP que vous fournissez (cf. `AGENTS.md` et `examples/backend/`) et
//! affiche la réponse en **fil de discussion** : questions (vert) et réponses
//! (crème) s'empilent, paginées — **Suite** = page suivante, **Retour** = page
//! précédente, **Sommaire** = nouveau fil, **Guide** = menu de services.
//!
//! Usage : `miniteld [device]`
//!
//! Variables d'environnement (toutes facultatives) :
//!
//! | Variable | Défaut | Rôle |
//! |---|---|---|
//! | `MINITEL_BACKEND` | `127.0.0.1:3009` | `ip:port` du backend HTTP |
//! | `MINITEL_TITLE` | `MINITEL` | titre affiché en rangée d'en-tête |
//! | `MINITEL_LOGO` | *(aucun)* | chemin d'un `.vtx` affiché à l'accueil |
//! | `MINITEL_SERVICES` | `services.json` | menu de la touche **Guide** |
//! | `MINITEL_CTRL_PORT` | `3010` | port de l'API de contrôle locale |
//! | `RUST_LOG` | `info` | verbosité (`tracing`) |

use std::env;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use minitel::constants::Color;
use minitel::edit::Editor;
use minitel::input::{Arrow, Decoder, Event, FnKey};
use minitel::link::{Link, LinkConfig, LinkEvent};
use minitel::backend;
use minitel::{protocol, videotex};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;

/// Port par défaut de l'API de contrôle (afficher du texte / une image depuis
/// une autre machine du LAN). Surchargeable par `MINITEL_CTRL_PORT`.
const DEFAULT_CTRL_PORT: u16 = 3010;
/// Adresse par défaut du backend applicatif. Surchargeable par `MINITEL_BACKEND`.
const DEFAULT_BACKEND: &str = "127.0.0.1:3009";
/// Titre par défaut de la rangée d'en-tête. Surchargeable par `MINITEL_TITLE`.
const DEFAULT_TITLE: &str = "MINITEL";
/// Largeur max du titre : au-delà, le pagineur « p n/m » déborderait des 40
/// colonnes de l'en-tête.
const TITLE_MAX: usize = 24;

/// Commande poussée par l'API de contrôle vers la boucle principale.
enum Ctl {
    Text(String),   // afficher du texte (encodé + paginé)
    Show(Vec<u8>),  // afficher un flux Vidéotex brut (image .vtx)
}

/// Délai d'inactivité (aucune frappe) avant de laisser le Minitel s'endormir :
/// on coupe alors la sonde + l'horloge (silence série total).
const IDLE_SLEEP_SECS: u64 = 180;

/// État partagé exposé par `GET /status`.
#[derive(Clone, Copy)]
struct Status {
    connected: bool,
    baud: u32,
    idle_secs: u64,
    sleeping: bool,
    net: backend::Net,
}

/// Période de sondage de l'état réseau (`GET /health` du backend).
/// Le backend met sa propre sonde Internet en cache 20 s : sonder plus vite ne
/// donnerait pas d'info plus fraîche.
const NET_POLL_SECS: u64 = 20;

const COLS: usize = 40;
const CONTENT_ROW: u8 = 3; // 1re ligne de contenu (réponse)
// --- mode chat : le champ de saisie est collé en **bas** de l'écran ---------
// Le fil se lit de haut en bas et la zone où l'on écrit reste au même endroit
// d'un écran à l'autre, comme dans n'importe quelle messagerie. Rangées :
//   1        en-tête (titre  p n/m)
//   3..20    fil de discussion (PAGE_ROWS lignes)
//   21       libellé « VOUS : » — sert aussi de ligne d'attente (spinner)
//   22..23   saisie (CHAT_INPUT_ROWS lignes)
//   24       pied de page (navigation)
const PAGE_ROWS: usize = 18; // lignes de fil par page
/// Longueur maximale du fil conservé. Au-delà on oublie les plus anciens
/// tours : le Minitel n'a pas de scrollback infini et le backend ne garde de
/// toute façon que les 6 derniers échanges comme contexte.
///
/// Exprimé en **pages entières** pour que la dernière page soit toujours pleine
/// (sinon le plafonnement laisse un reliquat et la 1re page affichée après
/// troncature est à moitié vide).
const MAX_TRANSCRIPT: usize = PAGE_ROWS * 14;
const FOOTER_ROW: u8 = 24; // derniere rangee : navigation
const CHAT_LABEL_ROW: u8 = 21; // libellé « VOUS : » / ligne d'attente
const CHAT_INPUT_ROW: u8 = 22; // 1re ligne de saisie en mode chat
const CHAT_INPUT_ROWS: usize = 1; // hauteur du champ (22 uniquement) — 23 = espace — 24 = pied de page

/// Version du daemon.
const VERSION: &str = "1.0";

const LABEL_ROW: u8 = 11; // ligne du libellé "RECHERCHE :"
const AREA_ROW: u8 = 12; // 1re ligne de la zone de saisie (multi-lignes)
const AREA_COLS: usize = 40; // largeur de la zone
const AREA_ROWS: usize = 4; // hauteur de la zone (lignes 12..15) — champ pointillé
const MSG_ROW: u8 = 16; // ligne des messages (erreurs) — juste sous la zone de saisie
const HINT_ROW: u8 = 18; // ligne de l'indice « GUIDE = services »
const ANIM_ROW: u8 = 20; // ligne de l'animation « recherche en cours »
const ANIM_COL: u8 = 9; // colonne du texte d'animation
/// Spinner façon shell : rotation entrecoupée d'une étincelle.
const SPIN: [u8; 8] = [b'|', b'*', b'/', b'*', b'-', b'*', b'\\', b'*'];

enum Mode {
    Home,
    Searching,
    Chat,   // réponse affichée + champ de suite (conversation)
    Guide,  // page Services (menu numéroté)
    Pushed, // contenu poussé par l'API de contrôle (image)
}

/// Une entrée du menu **Services** (touche Guide).
///
/// Le menu est **entièrement à vous** : il est lu au démarrage depuis un fichier
/// JSON (`MINITEL_SERVICES`, défaut `services.json`) — un tableau d'objets
/// `{"key": "1", "name": "meteo", "label": "Meteo du jour"}`. `name` est envoyé
/// tel quel au backend (`GET /service?name=meteo`), `label` est ce que lit
/// l'utilisateur. Voir `services.example.json`.
#[derive(Debug, Clone, Deserialize)]
struct Service {
    /// Touche à taper (un seul caractère ; les entrées invalides sont ignorées).
    key: char,
    /// Identifiant passé au backend.
    name: String,
    /// Libellé affiché (tronqué à 34 colonnes à l'affichage).
    label: String,
}

/// Charge le menu Services. Fichier absent → menu vide (la touche Guide affiche
/// alors une page « aucun service »), ce qui est le cas par défaut : un menu
/// codé en dur n'aurait aucun sens pour le backend de quelqu'un d'autre.
fn load_services(path: &str) -> Vec<Service> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            info!(path, error = %e, "pas de menu de services (touche Guide vide)");
            return Vec::new();
        }
    };
    match serde_json::from_str::<Vec<Service>>(&raw) {
        Ok(v) => {
            info!(path, count = v.len(), "menu de services chargé");
            v
        }
        Err(e) => {
            warn!(path, error = %e, "services.json illisible — menu ignoré");
            Vec::new()
        }
    }
}

/// Serveur HTTP de contrôle (LAN) : `GET /status`, `POST /text`, `POST /show`.
/// Hand-roll minimal (pas de dépendance HTTP) sur une écoute tokio.
async fn control_server(
    port: u16,
    ctl_tx: mpsc::Sender<Ctl>,
    status: Arc<Mutex<Status>>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!(port, "API de contrôle Minitel à l'écoute");
    loop {
        let (sock, _) = listener.accept().await?;
        let ctl = ctl_tx.clone();
        let st = status.clone();
        tokio::spawn(async move {
            let _ = handle_conn(sock, ctl, st).await;
        });
    }
}

async fn handle_conn(
    mut sock: tokio::net::TcpStream,
    ctl_tx: mpsc::Sender<Ctl>,
    status: Arc<Mutex<Status>>,
) -> std::io::Result<()> {
    // lit la requête jusqu'à la fin des en-têtes, puis le corps (Content-Length)
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    let head_end = loop {
        if let Some(i) = find_sub(&buf, b"\r\n\r\n") {
            break i;
        }
        let n = sock.read(&mut tmp).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > 1_000_000 {
            return Ok(()); // garde-fou
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.lines();
    let req = lines.next().unwrap_or("");
    let mut it = req.split_whitespace();
    let method = it.next().unwrap_or("");
    let path = it.next().unwrap_or("");
    let clen: usize = head
        .lines()
        .find_map(|l| {
            let l = l.to_ascii_lowercase();
            l.strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0))
        })
        .unwrap_or(0);
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < clen {
        let n = sock.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }

    let resp: Vec<u8> = match (method, path) {
        ("GET", "/status") => {
            let s = *status.lock().unwrap();
            let net = match s.net {
                backend::Net::Unknown => "unknown",
                backend::Net::Online => "online",
                backend::Net::NoWeb => "noweb",
                backend::Net::Offline => "offline",
            };
            let json = format!(
                "{{\"connected\":{},\"baud\":{},\"idle_secs\":{},\"sleeping\":{},\"net\":\"{}\"}}",
                s.connected, s.baud, s.idle_secs, s.sleeping, net
            );
            http_ok("application/json", json.as_bytes())
        }
        ("POST", "/text") => {
            let txt = String::from_utf8_lossy(&body).to_string();
            let _ = ctl_tx.send(Ctl::Text(txt)).await;
            http_ok("application/json", b"{\"ok\":true}")
        }
        ("POST", "/show") => {
            let _ = ctl_tx.send(Ctl::Show(body)).await;
            http_ok("application/json", b"{\"ok\":true}")
        }
        _ => b"HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
    };
    sock.write_all(&resp).await?;
    let _ = sock.flush().await;
    Ok(())
}

fn http_ok(ctype: &str, body: &[u8]) -> Vec<u8> {
    let mut v = format!(
        "HTTP/1.0 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    v.extend_from_slice(body);
    v
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

struct App {
    link: Link,
    dec: Decoder,
    ed: Editor, // éditeur de saisie multi-lignes
    lines: Vec<(Color, String)>, // fil de discussion mis à plat (questions + réponses)
    page: usize,                 // page affichée du fil
    pending_q: Option<String>,   // question en vol, à réafficher avec sa réponse
    mode: Mode,
    backend_addr: String, // `ip:port` du backend applicatif
    title: String,        // titre de la rangée d'en-tête (MINITEL_TITLE)
    logo: Option<Vec<u8>>, // flux Vidéotex affiché à l'accueil (MINITEL_LOGO)
    services: Vec<Service>, // menu de la touche Guide (services.json)
    baud: u32, // vitesse négociée (pour la barre de statut)
    spin: usize, // frame courante du spinner
    anim_col: u8, // position de la ligne d'attente (dépend de l'écran de départ)
    anim_row: u8,
    search_at: Instant, // début de la requête en cours (chrono d'attente)
    shown_secs: u64, // dernières secondes affichées (évite de réécrire à 120 ms)
    results_tx: mpsc::Sender<std::io::Result<String>>, // renvoi de la recherche async
    last_seen: Instant, // dernière preuve de vie du Minitel (Connected/Identify)
    last_activity: Instant, // dernière frappe réelle (pour la mise en veille)
    sleeping: bool, // sonde/horloge coupées (Minitel endormi)
    input_row: u8, // ligne de base du champ de saisie (accueil vs suite)
    net: backend::Net, // état de la chaîne réseau (voyant de la rangée 0)
}

/// Ligne de statut (rangée 0) : version, vitesse, **état réseau**, heure.
/// Cadrée sur **37 colonnes** : les dernières colonnes de la rangée 0 sont
/// réservées à l'indicateur matériel du Minitel (le « F » encadré).
///
/// `net.label()` fait toujours 6 caractères, donc l'heure reste calée à droite
/// quel que soit l'état.
fn status_line(baud: u32, net: backend::Net) -> String {
    const W: usize = 37;
    let t = chrono::Local::now().format("%H:%M").to_string();
    let base = format!("miniteld {VERSION}  {baud} bd  {}", net.label());
    let pad = W.saturating_sub(base.chars().count() + t.len());
    let mut s = format!("{base}{}{t}", " ".repeat(pad.max(1)));
    s.truncate(W);
    s
}

impl App {
    fn pages(&self) -> usize {
        self.lines.len().div_ceil(PAGE_ROWS).max(1)
    }

    fn last_page(&self) -> usize {
        self.pages() - 1
    }

    async fn send(&self, bytes: Vec<u8>) {
        let _ = self.link.send(bytes).await;
    }

    /// Barre de statut en rangée 0 (hors zone scrollable) : version/vitesse/heure.
    /// Ne touche pas au contenu de l'écran. Rétablit le curseur dans la zone de
    /// saisie si on est à l'accueil (sinon la rangée 0 « vole » le curseur).
    async fn draw_status(&self) {
        self.send(protocol::goto_row0()).await;
        self.send(videotex::encode_text(&status_line(self.baud, self.net))).await;
        if let Mode::Home = self.mode {
            self.place_cursor().await;
        }
    }

    /// Positionne le curseur matériel (clignotant) à la position de l'éditeur.
    async fn place_cursor(&self) {
        let (r, c) = self.ed.cursor_rc();
        self.send(protocol::move_to(1 + c as u8, self.input_row + r as u8)).await;
        self.send(protocol::cursor(true)).await;
    }

    /// Redessine les lignes de la zone de saisie de `from` à `upto` (incluses).
    /// Chaque ligne = texte puis **pointillés** jusqu'à 40 colonnes (le champ
    /// classique) ; le texte tapé « écrase » les points, qui reviennent à
    /// l'effacement. Puis replace le curseur.
    async fn redraw_input(&self, from: usize, upto: usize) {
        let lines = self.ed.lines();
        let empty = String::new();
        for i in from..=upto.min(AREA_ROWS - 1) {
            let l = lines.get(i).unwrap_or(&empty);
            let fill = AREA_COLS.saturating_sub(l.chars().count());
            let row_str = format!("{l}{}", ".".repeat(fill));
            self.send(protocol::move_to(1, self.input_row + i as u8)).await;
            self.send(videotex::encode_text(&row_str)).await;
        }
        self.place_cursor().await;
    }

    /// Écran d'accueil : bannière + « RECHERCHE : » + zone multi-lignes + curseur.
    async fn draw_home(&mut self) {
        self.mode = Mode::Home;
        self.ed.clear();
        self.ed.set_rows(AREA_ROWS); // zone de recherche : 4 lignes
        self.input_row = AREA_ROW;
        self.send(protocol::clear_screen()).await;
        self.draw_banner().await;
        self.send(protocol::move_to(2, LABEL_ROW)).await;
        self.send(videotex::colored(Color::Cyan, "RECHERCHE :")).await;
        self.redraw_input(0, AREA_ROWS - 1).await; // champ pointillé (4 lignes)
        if !self.services.is_empty() {
            self.send(protocol::move_to(1, HINT_ROW)).await;
            self.send(videotex::colored(Color::Green, "GUIDE = services")).await;
        }
        self.draw_status().await; // barre + curseur dans la zone
    }

    /// Bannière de l'accueil (lignes 2-9).
    ///
    /// Avec `MINITEL_LOGO`, on envoie le flux Vidéotex tel quel : c'est la voie
    /// noble (mosaïque G1, cf. `docs/image-conversion.md`). Sans logo, on se
    /// rabat sur le titre en double taille — aucun asset n'est donc requis pour
    /// démarrer, et le daemon n'embarque aucune image en dur.
    async fn draw_banner(&mut self) {
        if let Some(logo) = self.logo.clone() {
            self.send(logo).await;
            return;
        }
        // double largeur : chaque caractère consomme 2 colonnes → on centre sur 20
        let title: String = self.title.chars().take(20).collect();
        let col = 1 + (20u8.saturating_sub(title.chars().count() as u8)) / 2;
        self.send(protocol::move_to(col, 6)).await;
        self.send(videotex::size(minitel::videotex::Size::DoubleSize)).await;
        self.send(videotex::colored(Color::Yellow, &title)).await;
        self.send(videotex::size(minitel::videotex::Size::Normal)).await;
    }

    /// Nouvelle conversation : vide le fil côté backend + retour accueil.
    async fn new_conversation(&mut self) {
        backend::reset(&self.backend_addr).await;
        self.lines.clear(); // le fil affiché doit suivre le fil du backend
        self.page = 0;
        self.pending_q = None;
        self.draw_home().await;
    }

    /// Lance la question courante — **non bloquant** : la requête part dans une
    /// tâche de fond, l'écran reste animé (spinner) pendant l'attente Codex.
    /// `cont` = relance dans le fil de conversation (sinon nouvelle question).
    async fn submit(&mut self, cont: bool) {
        let q = self.ed.text().trim().to_string();
        if q.is_empty() {
            return;
        }
        info!(query = %q, cont, "question");
        // mémorisée pour être réaffichée au-dessus de la réponse (fil de discussion)
        self.pending_q = Some(q.clone());
        self.start_waiting("Recherche en cours").await;

        // recherche en tâche de fond → renvoi via le canal
        let tx = self.results_tx.clone();
        let auth = self.backend_addr.clone();
        tokio::spawn(async move {
            let r = backend::ask(&auth, &q, cont).await;
            let _ = tx.send(r).await;
        });
    }

    /// Page Services (menu numéroté façon serveur télématique).
    async fn draw_guide(&mut self) {
        self.mode = Mode::Guide;
        self.send(protocol::clear_screen()).await;
        self.send(protocol::cursor(false)).await;
        self.send(protocol::move_to(1, 1)).await;
        self.send(videotex::size(minitel::videotex::Size::DoubleHeight)).await;
        self.send(videotex::colored(Color::Yellow, "SERVICES")).await;
        self.send(videotex::size(minitel::videotex::Size::Normal)).await;
        if self.services.is_empty() {
            // Cas par défaut d'une installation neuve : on le dit franchement
            // plutôt que d'afficher un menu vide qui passerait pour un bug.
            self.send(protocol::move_to(3, 6)).await;
            self.send(videotex::colored(Color::White, "Aucun service configure.")).await;
            self.send(protocol::move_to(3, 8)).await;
            self.send(videotex::colored(Color::Cyan, "Voir services.example.json")).await;
        } else {
            let mut row = 5u8;
            for svc in self.services.iter() {
                // 2 lignes par entrée à partir de la 5 → on s'arrête avant le pied
                if row >= FOOTER_ROW - 1 {
                    break;
                }
                self.send(protocol::move_to(3, row)).await;
                self.send(videotex::colored(Color::White, &svc.key.to_string())).await;
                self.send(protocol::move_to(6, row)).await;
                let label: String = svc.label.chars().take(COLS - 6).collect();
                self.send(videotex::colored(Color::Cyan, &label)).await;
                row += 2;
            }
        }
        self.send(protocol::move_to(1, FOOTER_ROW)).await;
        self.send(videotex::colored(Color::Green, "Tapez un chiffre  SOMMAIRE=accueil")).await;
    }

    /// Charge un service (Guide) — non bloquant, spinner pendant l'attente.
    async fn run_service(&mut self, name: &str) {
        info!(service = name, "chargement service");
        self.start_waiting("Chargement").await;
        let tx = self.results_tx.clone();
        let auth = self.backend_addr.clone();
        let name = name.to_string();
        tokio::spawn(async move {
            let r = backend::service(&auth, &name).await;
            let _ = tx.send(r).await;
        });
    }

    /// Avance le spinner d'une frame (redessine juste le caractère) et met à
    /// jour le chrono d'attente.
    ///
    /// Le chrono n'est pas cosmétique : un simple spinner ne dit pas si ça
    /// avance ou si c'est planté. Les secondes qui défilent, si.
    async fn tick_spinner(&mut self) {
        self.spin = (self.spin + 1) % SPIN.len();
        // le spinner est juste après le libellé, cadré sur 20 caractères
        self.send(protocol::move_to(self.anim_col + 20, self.anim_row)).await;
        self.send(vec![SPIN[self.spin]]).await;
        // secondes écoulées — réécrites seulement quand elles changent (le tick
        // est à 120 ms : sinon on saturerait la liaison série à 4800 bauds)
        let secs = self.search_at.elapsed().as_secs();
        if secs != self.shown_secs {
            self.shown_secs = secs;
            self.send(protocol::move_to(self.anim_col + 22, self.anim_row)).await;
            self.send(videotex::encode_text(&format!("{secs:>3}s"))).await;
        }
    }

    /// Prépare l'écran d'attente (libellé + spinner + chrono à zéro).
    ///
    /// La ligne d'attente dépend de l'écran de départ : à l'accueil elle a sa
    /// place dédiée (`ANIM_ROW`), mais en mode chat cette rangée fait partie du
    /// fil — on écrirait par-dessus la discussion. On réutilise donc la ligne du
    /// libellé « VOUS : », juste au-dessus de la saisie : c'est là que l'œil est,
    /// et `draw_answer` la réécrira de toute façon en affichant la réponse.
    async fn start_waiting(&mut self, label: &str) {
        (self.anim_col, self.anim_row) = match self.mode {
            Mode::Chat => (1, CHAT_LABEL_ROW),
            _ => (ANIM_COL, ANIM_ROW),
        };
        self.mode = Mode::Searching;
        self.spin = 0;
        self.search_at = Instant::now();
        self.shown_secs = 0;
        self.send(protocol::cursor(false)).await; // pas de curseur pendant l'attente
        self.send(protocol::move_to(self.anim_col, self.anim_row)).await;
        // libellé cadré sur 20 colonnes : tick_spinner écrit toujours en +20
        self.send(videotex::colored(Color::Cyan, &format!("{label:<20}"))).await;
        self.send(vec![SPIN[0]]).await;
        self.send(protocol::move_to(self.anim_col + 22, self.anim_row)).await;
        self.send(videotex::encode_text("  0s")).await;
    }

    /// Ajoute un tour au **fil de discussion** et affiche la dernière page.
    ///
    /// La question est reprise à l'écran (préfixe `>`, en vert) : sans elle on
    /// ne voit qu'une réponse hors contexte, et une relance du genre « et
    /// demain ? » devient illisible une fois la question effacée du champ.
    async fn push_turn(&mut self, question: Option<String>, answer: &str) {
        if !self.lines.is_empty() {
            self.lines.push((Color::White, String::new())); // séparateur de tours
        }
        if let Some(q) = question {
            // `COLS - 2` : la place du préfixe « > » / de son indentation
            for (i, l) in videotex::wrap(&q, COLS - 2).into_iter().enumerate() {
                let prefix = if i == 0 { "> " } else { "  " };
                self.lines.push((Color::Green, format!("{prefix}{l}")));
            }
        }
        self.lines.extend(wrap_answer(answer));
        // Le fil est borné : au-delà, on oublie les tours les plus anciens (le
        // backend ne garde que 6 échanges de contexte de toute façon).
        if self.lines.len() > MAX_TRANSCRIPT {
            let drop = self.lines.len() - MAX_TRANSCRIPT;
            self.lines.drain(..drop);
        }
        self.page = self.last_page(); // on arrive sur le tour qu'on vient d'avoir
        self.ed.clear();
        self.draw_answer().await;
    }

    /// Affiche un texte poussé par l'API de contrôle (encodé + paginé). Passe en
    /// Chat : on peut enchaîner en conversation ou revenir par Sommaire.
    async fn push_text(&mut self, text: String) {
        self.push_turn(None, &text).await;
    }

    /// Affiche un flux Vidéotex brut poussé par l'API de contrôle (image .vtx).
    async fn show_pushed(&mut self, bytes: Vec<u8>) {
        self.mode = Mode::Pushed;
        self.send(protocol::cursor(false)).await;
        self.send(protocol::clear_screen()).await;
        let mut clr0 = protocol::goto_row0();
        clr0.push(minitel::constants::CANCEL); // nettoie la rangée 0
        self.send(clr0).await;
        self.send(bytes).await;
    }

    /// Traite le résultat de la recherche (reçu du canal).
    async fn on_result(&mut self, r: std::io::Result<String>) {
        match r {
            Ok(text) if !text.trim().is_empty() => {
                let q = self.pending_q.take();
                self.push_turn(q, &text).await; // passe en mode Chat
            }
            Ok(_) => self.message("Aucune reponse.").await,
            Err(e) => {
                warn!(error = %e, net = ?self.net, "recherche en échec");
                // message adapté au maillon cassé (NAS injoignable / pas d'accès
                // Internet / backend en erreur) plutôt qu'un générique
                let hint = self.net.failure_hint();
                self.message(hint).await;
            }
        }
    }

    /// Affiche le fil de discussion (page courante) + le champ de saisie.
    async fn draw_answer(&mut self) {
        self.mode = Mode::Chat;
        // Borne la saisie à 1 ligne avec un espace avant le pied de page. Le champ
        // est collé en bas, une 2e ligne mangerait le pied de page.
        self.ed.set_rows(CHAT_INPUT_ROWS);
        self.input_row = CHAT_INPUT_ROW;
        self.send(protocol::clear_screen()).await;
        self.send(protocol::move_to(1, 1)).await;
        let header = format!("{}   p{}/{}", self.title, self.page + 1, self.pages());
        self.send(videotex::colored(Color::Yellow, &header)).await;

        // fil de discussion (page courante)
        let start = self.page * PAGE_ROWS;
        let mut row = CONTENT_ROW;
        for (color, text) in self.lines.iter().skip(start).take(PAGE_ROWS) {
            self.send(protocol::move_to(1, row)).await;
            self.send(videotex::colored(*color, text)).await;
            row += 1;
        }
        // champ de saisie (1 ligne de pointillés) + curseur
        self.send(protocol::move_to(1, CHAT_INPUT_ROW - 1)).await;
        self.send(videotex::colored(Color::Cyan, "VOUS :")).await;
        self.redraw_input(0, CHAT_INPUT_ROWS - 1).await;
        // Pied de page **fixe** : 38 caractères. La séquence de couleur occupe
        // elle-même une case écran en Vidéotex, donc 40 caractères de texte
        // débordent sur la ligne suivante — d'où la marge.
        self.send(protocol::move_to(1, FOOTER_ROW)).await;
        self.send(videotex::colored(Color::Green, "ENVOI  SUITE/RETOUR=page  SOMMAIRE=raz"))
            .await;
        self.place_cursor().await;
    }

    /// Redessine l'accueil et affiche un message (erreur / info) sous la zone.
    ///
    /// Le message a sa **propre ligne** (`MSG_ROW`, sous le champ de saisie) et
    /// occupe la largeur entière, complétée par des espaces : sans ce padding,
    /// un message court laisserait dépasser la queue d'un texte plus long déjà
    /// présent sur la ligne. C'était le bug historique — `MSG_ROW` valait 18,
    /// soit la ligne de l'indice « GUIDE = services », qui restait visible
    /// derrière l'erreur.
    async fn message(&mut self, msg: &str) {
        self.draw_home().await;
        self.send(protocol::move_to(1, MSG_ROW)).await;
        let clipped: String = msg.chars().take(COLS - 1).collect();
        let padded = format!("{clipped:<width$}", width = COLS - 1);
        self.send(videotex::colored(Color::White, &padded)).await;
        self.place_cursor().await; // curseur de retour dans la zone de saisie
    }

    async fn handle(&mut self, ev: Event) {
        // toute frappe réelle (pas les accusés/sondes) compte comme activité
        if matches!(
            ev,
            Event::Char(_) | Event::Enter | Event::Function(_) | Event::Arrow(_)
        ) {
            self.last_activity = Instant::now();
        }
        match (&self.mode, ev) {
            // --- saisie (accueil OU champ de suite en conversation) ---
            (Mode::Home | Mode::Chat, Event::Char(c)) => {
                let append = self.ed.at_end(); // ajout en fin = écho direct (pas de repaint)
                if self.ed.insert(c) {
                    if append {
                        // le caractère écrase le point ; le curseur matériel avance seul
                        self.send(videotex::encode_text(&c.to_string())).await;
                        // franchissement de ligne → recaler proprement le curseur
                        if self.ed.cursor_col() == 0 {
                            self.place_cursor().await;
                        }
                    } else {
                        // insertion au milieu → repaint des lignes impactées
                        let (row, col) = self.ed.cursor_rc();
                        let from = if col == 0 { row.saturating_sub(1) } else { row };
                        self.redraw_input(from, self.ed.last_row()).await;
                    }
                }
            }
            (Mode::Home | Mode::Chat, Event::Function(FnKey::Correction)) => {
                let at_end = self.ed.at_end();
                let col_before = self.ed.cursor_col();
                let old_last = self.ed.last_row();
                if self.ed.backspace() {
                    if at_end && col_before > 0 {
                        // effacement en fin de ligne : BS, remet un point, BS (sur le point)
                        self.send(vec![0x08, b'.', 0x08]).await;
                    } else {
                        // milieu / franchissement de ligne → repaint
                        let (row, _) = self.ed.cursor_rc();
                        self.redraw_input(row, old_last).await;
                    }
                }
            }
            (Mode::Home | Mode::Chat, Event::Function(FnKey::Cancel)) => {
                let old_last = self.ed.last_row();
                self.ed.clear();
                self.redraw_input(0, old_last).await;
            }
            (Mode::Home | Mode::Chat, Event::Arrow(a)) => {
                match a {
                    Arrow::Left => self.ed.left(),
                    Arrow::Right => self.ed.right(),
                    Arrow::Up => self.ed.up(),
                    Arrow::Down => self.ed.down(),
                }
                self.place_cursor().await; // pas de redraw : juste le curseur
            }
            // --- page Services (Guide) : sélection par chiffre ---
            (Mode::Guide, Event::Char(c)) => {
                if let Some(name) =
                    self.services.iter().find(|s| s.key == c).map(|s| s.name.clone())
                {
                    self.run_service(&name).await;
                }
            }
            // Touche Guide → ouvre la page Services (depuis accueil ou conversation)
            (Mode::Home | Mode::Chat, Event::Function(FnKey::Guide)) => self.draw_guide().await,
            // Envoi : nouvelle question (accueil) ou relance dans le fil (chat)
            (Mode::Home, Event::Enter | Event::Function(FnKey::Send)) => self.submit(false).await,
            (Mode::Chat, Event::Enter | Event::Function(FnKey::Send)) => self.submit(true).await,
            // Suite / Retour = navigation dans le fil (le champ de saisie est
            // préservé : on peut relire l'historique en cours de frappe)
            (Mode::Chat, Event::Function(FnKey::Next)) => {
                if self.page + 1 < self.pages() {
                    self.page += 1;
                    self.draw_answer().await;
                }
            }
            (Mode::Chat, Event::Function(FnKey::Return)) => {
                if self.page > 0 {
                    self.page -= 1;
                    self.draw_answer().await;
                }
            }
            // Connexion/Fin = hard reset : renégocie le lien (→ redessine l'accueil).
            (_, Event::Function(FnKey::Connect)) => {
                info!("Connexion/Fin → reset du lien");
                self.link.reset().await;
                self.last_seen = Instant::now();
            }
            // --- pendant la recherche : on ignore le reste du clavier ---
            (Mode::Searching, _) => {}
            // --- Sommaire = nouvelle conversation (vide le fil) + accueil ---
            (_, Event::Function(FnKey::Summary)) => self.new_conversation().await,
            (_, Event::Identify { constructor, device, version }) => {
                self.last_seen = Instant::now(); // preuve de vie (sonde watchdog)
                debug!(
                    constructor = %(constructor as char),
                    device = %(device as char),
                    version = %(version as char),
                    "Minitel identifié"
                );
            }
            (_, Event::ProtocolAck(_)) => {}
            (_, ev) => debug!(?ev, "événement ignoré"),
        }
    }
}

/// Met une réponse (déjà Vidéotex-compatible) à plat en lignes ≤ 40 colonnes.
///
/// Tout en crème : dans un fil de discussion, la couleur distingue le
/// **locuteur** (vert pour les questions, crème pour les réponses) — colorer en
/// plus la 1re ligne de chaque réponse la ferait passer pour un titre.
fn wrap_answer(text: &str) -> Vec<(Color, String)> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let raw = raw.trim_end();
        if raw.is_empty() {
            out.push((Color::White, String::new()));
            continue;
        }
        for l in videotex::wrap(raw, COLS) {
            out.push((Color::White, l));
        }
    }
    out
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let mut cfg = LinkConfig::default();
    if let Some(dev) = env::args().nth(1) {
        cfg.device = dev.into();
    }
    let backend_addr = env::var("MINITEL_BACKEND").unwrap_or_else(|_| DEFAULT_BACKEND.into());
    let title: String = env::var("MINITEL_TITLE")
        .unwrap_or_else(|_| DEFAULT_TITLE.into())
        .chars()
        .take(TITLE_MAX)
        .collect();
    let services = load_services(
        &env::var("MINITEL_SERVICES").unwrap_or_else(|_| "services.json".into()),
    );
    // Logo facultatif : un `.vtx` illisible ne doit pas empêcher le démarrage —
    // on trace et on retombe sur la bannière texte.
    let logo = match env::var("MINITEL_LOGO") {
        Ok(path) => match std::fs::read(&path) {
            Ok(bytes) => {
                info!(path, bytes = bytes.len(), "logo chargé");
                Some(bytes)
            }
            Err(e) => {
                warn!(path, error = %e, "logo illisible — bannière texte");
                None
            }
        },
        Err(_) => None,
    };
    let ctrl_port = env::var("MINITEL_CTRL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_CTRL_PORT);
    info!(device = %cfg.device.display(), %backend_addr, %title, "démarrage miniteld");

    let (results_tx, mut results_rx) = mpsc::channel::<std::io::Result<String>>(4);
    // API de contrôle (afficher du texte / une image depuis le LAN)
    let (ctl_tx, mut ctl_rx) = mpsc::channel::<Ctl>(8);
    let status = Arc::new(Mutex::new(Status {
        connected: false,
        baud: 0,
        idle_secs: 0,
        sleeping: false,
        net: backend::Net::Unknown,
    }));
    tokio::spawn(control_server(ctrl_port, ctl_tx, status.clone()));

    // Sonde réseau : interroge `GET /health` du backend en boucle et ne signale
    // que les **changements** d'état (inutile de redessiner la barre sinon).
    let (net_tx, mut net_rx) = mpsc::channel::<backend::Net>(4);
    {
        let authority = backend_addr.clone();
        tokio::spawn(async move {
            let mut last = backend::Net::Unknown;
            let mut tick = tokio::time::interval(Duration::from_secs(NET_POLL_SECS));
            loop {
                tick.tick().await;
                let now = backend::health(&authority).await;
                if now != last {
                    info!(from = ?last, to = ?now, "état réseau");
                    last = now;
                    if net_tx.send(now).await.is_err() {
                        break; // boucle principale terminée
                    }
                }
            }
        });
    }

    let mut app = App {
        link: Link::spawn(cfg),
        dec: Decoder::new(),
        ed: Editor::new(AREA_COLS, AREA_ROWS),
        lines: Vec::new(),
        pending_q: None,
        page: 0,
        mode: Mode::Home,
        backend_addr,
        title,
        logo,
        services,
        baud: 1200,
        spin: 0,
        anim_col: ANIM_COL,
        anim_row: ANIM_ROW,
        search_at: Instant::now(),
        shown_secs: 0,
        results_tx,
        last_seen: Instant::now(),
        last_activity: Instant::now(),
        sleeping: false,
        input_row: AREA_ROW,
        net: backend::Net::Unknown,
    };

    let mut sigint = Box::pin(tokio::signal::ctrl_c());
    let mut clock = tokio::time::interval(Duration::from_secs(60));
    let mut spinner = tokio::time::interval(Duration::from_millis(120));
    let mut watchdog = tokio::time::interval(Duration::from_secs(4));
    loop {
        tokio::select! {
            _ = &mut sigint => { info!("arrêt demandé"); break; }
            _ = clock.tick() => {
                // horloge de la barre de statut — pas en veille (silence série)
                if app.baud > 0 && !app.sleeping { app.draw_status().await; }
            }
            _ = watchdog.tick() => {
                let inactive = app.last_activity.elapsed().as_secs() >= IDLE_SLEEP_SECS;
                // état exposé pour /status
                if let Ok(mut s) = status.lock() {
                    s.baud = app.baud;
                    s.idle_secs = app.last_activity.elapsed().as_secs();
                    s.sleeping = app.sleeping;
                    s.net = app.net;
                    if !app.sleeping {
                        s.connected = app.baud > 0 && app.last_seen.elapsed().as_secs() < 8;
                    } // en veille : on fige `connected` (dernier état connu)
                }
                if inactive {
                    // veille : silence total (ni sonde ni horloge) → le Minitel s'endort
                    if !app.sleeping {
                        app.sleeping = true;
                        info!("inactivité → veille (sonde coupée, écran s'éteindra)");
                    }
                } else if matches!(app.mode, Mode::Searching) {
                    // recherche en cours : on ne sonde pas (le Minitel n'a rien à
                    // dire et l'écran est occupé) — mais il faut garder
                    // `last_seen` frais, sinon au retour en mode Chat le watchdog
                    // voit un silence de 30-50 s qu'il a lui-même causé et
                    // réinitialise le lien → l'accueil s'affiche par-dessus la
                    // réponse tout juste rendue. Absence de sonde ≠ Minitel muet.
                    app.last_seen = Instant::now();
                } else if app.baud > 0 {
                    // actif : sonde de présence + reset si muet (power-cycle)
                    app.send(protocol::identify_request()).await;
                    if app.last_seen.elapsed() > Duration::from_secs(10) {
                        info!("watchdog: Minitel muet → reset du lien");
                        app.link.reset().await;
                        app.last_seen = Instant::now();
                    }
                }
            }
            Some(n) = net_rx.recv() => {
                app.net = n;
                // rafraîchit le voyant tout de suite — sauf en veille, où toute
                // écriture série réveillerait le Minitel
                if app.baud > 0 && !app.sleeping { app.draw_status().await; }
            }
            Some(c) = ctl_rx.recv() => {
                match c {
                    Ctl::Text(t) => app.push_text(t).await,
                    Ctl::Show(b) => app.show_pushed(b).await,
                }
            }
            _ = spinner.tick() => {
                if let Mode::Searching = app.mode { app.tick_spinner().await; }
            }
            Some(r) = results_rx.recv() => {
                if let Mode::Searching = app.mode { app.on_result(r).await; }
            }
            evt = app.link.recv() => {
                match evt {
                    Some(LinkEvent::Connected(baud)) => {
                        info!(baud, "Minitel connecté");
                        app.baud = baud;
                        app.last_seen = Instant::now();
                        app.last_activity = Instant::now(); // repart actif
                        app.sleeping = false;
                        app.send(protocol::set_echo(false)).await;
                        // le 1B repart toujours en majuscules → repasser en
                        // minuscules à chaque (re)connexion (option non mémorisée)
                        app.send(protocol::keyboard_lowercase()).await;
                        app.draw_home().await;
                        app.send(protocol::identify_request()).await;
                    }
                    Some(LinkEvent::Disconnected) => warn!("Minitel déconnecté (reconnexion auto)"),
                    Some(LinkEvent::Rx(bytes)) => {
                        if app.sleeping {
                            // réveil : toute activité entrante → renégocie (gère un
                            // power-cycle survenu pendant le sommeil) + redessine l'accueil
                            info!("réveil Minitel → renégociation");
                            app.sleeping = false;
                            app.last_activity = Instant::now();
                            app.dec = Decoder::new(); // octets de réveil ignorés
                            app.link.reset().await;
                        } else {
                            for ev in app.dec.push(&bytes) {
                                app.handle(ev).await;
                            }
                        }
                    }
                    None => { warn!("lien terminé"); break; }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le pied de page du fil est écrit avec un attribut de couleur, et en
    /// Vidéotex cet attribut **occupe une case écran** : 40 caractères de texte
    /// déborderaient donc sur la ligne suivante. C'était le cas de l'ancien
    /// libellé « ENVOI=suite SUITE=page+ SOMMAIRE=nouveau » (pile 40).
    #[test]
    fn chat_footer_fits_with_color_attribute() {
        const FOOTER: &str = "ENVOI  SUITE/RETOUR=page  SOMMAIRE=raz";
        assert!(FOOTER.chars().count() + 1 <= COLS, "{} cols", FOOTER.chars().count());
    }

    /// Le mode chat empile fil → libellé → saisie → pied de page. Aucune de ces
    /// zones ne doit se chevaucher ni dépasser la 24e rangée : écrire **sous** la
    /// dernière ligne fait défiler tout l'écran du Minitel, ce qui décale la
    /// mise en page pour de bon.
    #[test]
    fn chat_layout_fits_the_screen() {
        let last_thread_row = CONTENT_ROW as usize + PAGE_ROWS - 1;
        assert!(last_thread_row < CHAT_LABEL_ROW as usize, "le fil mange le libelle");
        assert_eq!(CHAT_LABEL_ROW + 1, CHAT_INPUT_ROW, "libelle non colle a la saisie");
        let last_input_row = CHAT_INPUT_ROW as usize + CHAT_INPUT_ROWS - 1;
        assert!(last_input_row < FOOTER_ROW as usize, "la saisie mange le pied de page");
        assert_eq!(FOOTER_ROW, 24, "le pied de page doit rester la derniere rangee");
    }

    /// La saisie du chat est bornée à sa hauteur visible. L'éditeur est créé
    /// avec la zone de l'accueil (4 lignes) : sans `set_rows`, une longue frappe
    /// remplirait 160 caractères et déborderait sur les rangées 24 et 25.
    #[test]
    fn chat_input_cannot_overflow_below_the_screen() {
        let mut ed = Editor::new(AREA_COLS, AREA_ROWS);
        ed.set_rows(CHAT_INPUT_ROWS);
        for _ in 0..500 {
            ed.insert('x');
        }
        assert_eq!(ed.last_row(), CHAT_INPUT_ROWS - 1);
        assert_eq!(ed.text().chars().count(), AREA_COLS * CHAT_INPUT_ROWS);
    }

    /// Le fil est borné : au-delà de `MAX_TRANSCRIPT`, les tours les plus
    /// anciens disparaissent, mais la page courante doit rester valide (sinon
    /// on afficherait une page vide après une longue conversation).
    #[test]
    fn transcript_stays_paginable_when_capped() {
        let lines: Vec<(Color, String)> =
            (0..MAX_TRANSCRIPT).map(|i| (Color::White, format!("l{i}"))).collect();
        let pages = lines.len().div_ceil(PAGE_ROWS).max(1);
        // pages entières : pas de reliquat qui donnerait une page à moitié vide
        assert_eq!(MAX_TRANSCRIPT % PAGE_ROWS, 0, "plafond non multiple d'une page");
        assert_eq!(pages, MAX_TRANSCRIPT / PAGE_ROWS);
        // la dernière page est toujours indexable
        assert!((pages - 1) * PAGE_ROWS < lines.len());
    }

    /// La rangée 0 fait 37 colonnes utiles (les dernières sont réservées à
    /// l'indicateur matériel du Minitel). Quel que soit l'état réseau ou la
    /// vitesse, la ligne ne doit jamais déborder — sinon elle mange le « F »
    /// encadré, voire provoque un retour à la ligne.
    #[test]
    fn status_line_never_exceeds_37_cols() {
        for net in [
            backend::Net::Unknown,
            backend::Net::Online,
            backend::Net::NoWeb,
            backend::Net::Offline,
        ] {
            for baud in [1200u32, 4800] {
                let s = status_line(baud, net);
                assert!(s.chars().count() <= 37, "{net:?} {baud}: {s:?}");
                assert!(s.contains(net.label()), "voyant absent : {s:?}");
            }
        }
    }

    /// Le voyant et l'heure doivent coexister : c'est tout l'intérêt d'un
    /// libellé de largeur fixe.
    #[test]
    fn status_line_keeps_clock_visible() {
        let s = status_line(4800, backend::Net::Online);
        let clock = chrono::Local::now().format("%H:%M").to_string();
        assert!(s.ends_with(&clock), "heure tronquee : {s:?}");
    }

    /// Régression : la ligne des messages ne doit pas être celle de l'indice
    /// « GUIDE = services », sinon l'erreur s'affiche par-dessus un texte
    /// existant et on lit un mélange des deux.
    #[test]
    fn message_row_does_not_collide_with_other_rows() {
        assert_ne!(MSG_ROW, HINT_ROW);
        assert_ne!(MSG_ROW, ANIM_ROW);
        assert_ne!(MSG_ROW, LABEL_ROW);
        // ni dans la zone de saisie (lignes AREA_ROW..AREA_ROW+AREA_ROWS-1)
        assert!(MSG_ROW < AREA_ROW || MSG_ROW >= AREA_ROW + AREA_ROWS as u8);
    }
}
