//! `minitel-demo` — mode démo Vidéotex pour valider le rendu sur le 1B.
//!
//! Un menu ; on tape un chiffre **0-9** → l'écran de démo correspondant ;
//! **Sommaire** = retour au menu. But : voir à l'œil ce que le Minitel rend
//! vraiment (couleurs, accents, attributs, tailles, mosaïque, fonds, REP…),
//! pour ensuite durcir `videotex.rs` selon les résultats.
//!
//! Usage : `minitel-demo [device]`

use std::env;

use minitel::constants::Color;
use minitel::input::{Decoder, Event, FnKey};
use minitel::link::{Link, LinkConfig, LinkEvent};
use minitel::videotex::Size;
use minitel::{protocol, videotex};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

/// Petit builder d'écran (accumule des octets Vidéotex).
struct Buf(Vec<u8>);
impl Buf {
    fn new() -> Self {
        Buf(protocol::clear_screen())
    }
    fn at(mut self, col: u8, row: u8) -> Self {
        self.0.extend(protocol::move_to(col, row));
        self
    }
    fn t(mut self, s: &str) -> Self {
        self.0.extend(videotex::encode_text(s));
        self
    }
    fn fg(mut self, c: Color, s: &str) -> Self {
        self.0.extend(videotex::colored(c, s));
        self
    }
    fn raw(mut self, b: Vec<u8>) -> Self {
        self.0.extend(b);
        self
    }
    fn byte(mut self, b: u8) -> Self {
        self.0.push(b);
        self
    }
    fn done(self) -> Vec<u8> {
        self.0
    }
}

const TITLES: [&str; 10] = [
    "Damier & degrades (mosaique+REP)",
    "Couleurs (8 gris)",
    "Accents & caracteres G2",
    "Attributs: clign/inv/souligne",
    "Tailles: double H / L / taille",
    "Mosaique G1 (blocs 2x3)",
    "Fonds colores (delimiteur)",
    "Titre (double taille)",
    "Rangee 0 (barre de statut)",
    "Repetition REP (aplats)",
];

/// Titre en haut + rappel navigation en bas.
fn frame(title: &str) -> Buf {
    Buf::new()
        .at(1, 1)
        .fg(Color::Yellow, title)
        .at(1, 24)
        .fg(Color::Green, "0-9=demo  SOMMAIRE=menu")
}

fn menu() -> Vec<u8> {
    let mut b = Buf::new()
        .at(1, 1)
        .raw(videotex::size(Size::DoubleHeight))
        .fg(Color::White, "DEMOS VIDEOTEX")
        .raw(videotex::size(Size::Normal));
    for (i, t) in TITLES.iter().enumerate() {
        b = b.at(1, 4 + i as u8).fg(Color::Cyan, &format!("{i}  {t}"));
    }
    b.at(1, 24).fg(Color::Green, "Tape un chiffre 0-9").done()
}

fn demo(n: u8) -> Vec<u8> {
    match n {
        1 => {
            // 8 couleurs (niveaux de gris sur écran mono)
            let names = [
                (Color::Black, "noir"),
                (Color::Blue, "bleu"),
                (Color::Red, "rouge"),
                (Color::Magenta, "magenta"),
                (Color::Green, "vert"),
                (Color::Cyan, "cyan"),
                (Color::Yellow, "jaune"),
                (Color::White, "blanc"),
            ];
            let mut b = frame(TITLES[1]);
            for (i, (c, name)) in names.iter().enumerate() {
                b = b.at(3, 4 + i as u8).fg(*c, &format!("####### {name}"));
            }
            b.done()
        }
        2 => frame(TITLES[2])
            .at(3, 5)
            .t("Accents: ete a l'ecole, ca va ?")
            .at(3, 6)
            .t("e a i o u -> é à î ô û  ç ê ë")
            .at(3, 9)
            .t("G2: £ ° ± ½ ¼ ¾ § oe OE")
            .at(3, 10)
            .t("Fleches: <- ^ -> v  (← ↑ → ↓)")
            .done(),
        3 => frame(TITLES[3])
            .at(3, 6)
            .t("Normal ")
            .raw(videotex::blink(true))
            .t("CLIGNOTE")
            .raw(videotex::blink(false))
            .at(3, 9)
            .t(" ")
            .raw(videotex::invert(true))
            .t(" INVERSE VIDEO ")
            .raw(videotex::invert(false))
            .at(3, 12)
            .t(" ")
            .raw(videotex::underline(true))
            .t(" souligne ")
            .raw(videotex::underline(false))
            .done(),
        4 => frame(TITLES[4])
            .at(3, 6)
            .raw(videotex::size(Size::DoubleHeight))
            .fg(Color::White, "Double hauteur")
            .raw(videotex::size(Size::Normal))
            .at(3, 10)
            .raw(videotex::size(Size::DoubleWidth))
            .fg(Color::Cyan, "Double largeur")
            .raw(videotex::size(Size::Normal))
            .at(3, 15)
            .raw(videotex::size(Size::DoubleSize))
            .fg(Color::Yellow, "DOUBLE")
            .raw(videotex::size(Size::Normal))
            .done(),
        5 => {
            // Mosaïque G1 : vocabulaire de blocs + une forme dessinée.
            let mut b = frame(TITLES[5]).at(3, 4).t("Blocs 2x3 isoles:");
            // les 6 blocs isolés
            b = b.at(3, 6).byte(videotex::g1());
            for bit in 0..6u8 {
                b = b.byte(videotex::mosaic(1 << bit)).byte(b' '); // bloc + espace G1
            }
            b = b.byte(videotex::g0());
            // une forme pleine + un cadre
            b = b
                .at(3, 9)
                .t("Plein / demi / cadre:")
                .at(3, 11)
                .byte(videotex::g1())
                .byte(videotex::mosaic(0x3F)) // plein
                .byte(videotex::mosaic(0x15)) // colonne gauche (b0,b2,b4)
                .byte(videotex::mosaic(0x2A)) // colonne droite (b1,b3,b5)
                .byte(videotex::g0());
            b.done()
        }
        6 => {
            // Fonds colorés : attribut de zone → espace délimiteur.
            let mut b = frame(TITLES[6]).at(3, 4).t("Fond = attribut zone (delim.)");
            let rows = [
                (Color::Red, "fond rouge"),
                (Color::Blue, "fond bleu"),
                (Color::Green, "fond vert"),
                (Color::White, "fond blanc"),
            ];
            for (i, (c, label)) in rows.iter().enumerate() {
                b = b
                    .at(3, 6 + 2 * i as u8)
                    .raw(videotex::bg(*c))
                    .t(" ") // délimiteur qui valide le fond
                    .fg(Color::Black, &format!("{label}       "))
                    .raw(videotex::bg(Color::Black))
                    .t(" ");
            }
            b.done()
        }
        7 => frame(TITLES[7])
            .at(6, 8)
            .raw(videotex::size(Size::DoubleSize))
            .fg(Color::White, "K L E K")
            .raw(videotex::size(Size::Normal))
            .at(3, 20)
            .fg(Color::Cyan, "(version mosaique 2x3 a venir)")
            .done(),
        8 => Buf::new()
            .raw(protocol::goto_row0())
            .fg(Color::White, " MINITEL             RANGEE 0 (statut) ")
            .at(1, 1)
            .fg(Color::Yellow, TITLES[8])
            .at(3, 6)
            .t("La ligne du haut est la rangee 0")
            .at(3, 8)
            .t("hors zone scrollable (HUD).")
            .at(1, 24)
            .fg(Color::Green, "0-9=demo  SOMMAIRE=menu")
            .done(),
        9 => {
            // Répétition REP : aplats horizontaux.
            let mut b = frame(TITLES[9]).at(3, 4).t("Barres via REP (0x12):");
            let bars = [
                (Color::Red, b'#'),
                (Color::Green, b'='),
                (Color::Yellow, b'*'),
                (Color::Cyan, b'-'),
            ];
            for (i, (c, ch)) in bars.iter().enumerate() {
                b = b
                    .at(3, 6 + 2 * i as u8)
                    .raw(videotex::fg(*c))
                    .byte(*ch)
                    .raw(videotex::rep(34)); // 1 + 34 = 35 colonnes
            }
            b.done()
        }
        0 => {
            // Damier mosaïque + dégradé.
            let mut b = frame(TITLES[0]).at(3, 4).t("Damier mosaique:");
            for row in 0..6u8 {
                b = b.at(3, 6 + row).byte(videotex::g1());
                for _ in 0..18 {
                    let p = if row % 2 == 0 { 0x3F } else { 0x00 };
                    b = b.byte(videotex::mosaic(p)).byte(videotex::mosaic(p ^ 0x3F));
                }
                b = b.byte(videotex::g0());
            }
            b.done()
        }
        _ => menu(),
    }
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
    info!(device = %cfg.device.display(), "démarrage minitel-demo");

    let mut link = Link::spawn(cfg);
    let mut dec = Decoder::new();
    let mut sigint = Box::pin(tokio::signal::ctrl_c());

    loop {
        tokio::select! {
            _ = &mut sigint => break,
            evt = link.recv() => match evt {
                Some(LinkEvent::Connected(_)) => {
                    info!("connecté → menu");
                    let _ = link.send(protocol::set_echo(false)).await;
                    let _ = link.send(protocol::cursor(false)).await;
                    let _ = link.send(menu()).await;
                }
                Some(LinkEvent::Disconnected) => warn!("déconnecté (reconnexion auto)"),
                Some(LinkEvent::Rx(bytes)) => {
                    for ev in dec.push(&bytes) {
                        match ev {
                            Event::Char(c @ '0'..='9') => {
                                let n = c as u8 - b'0';
                                info!(demo = n, title = TITLES[n as usize], "affichage démo");
                                let _ = link.send(demo(n)).await;
                            }
                            Event::Function(FnKey::Summary) => {
                                let _ = link.send(menu()).await;
                            }
                            _ => {}
                        }
                    }
                }
                None => break,
            }
        }
    }
}
