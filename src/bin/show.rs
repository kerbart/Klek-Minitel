//! `minitel-show` — affiche un flux Vidéotex brut (`.vtx`) sur le Minitel.
//!
//! Le fichier `.vtx` contient les octets Vidéotex prêts à l'emploi (produits
//! par l'outil `mosaic-conv` sur le NAS). Le viewer se contente d'initialiser
//! le terminal, d'effacer l'écran et d'envoyer le flux — puis reste en vie
//! (l'image tient à l'écran, et le lien se reconnecte tout seul si besoin).
//!
//! Usage : `minitel-show <fichier.vtx> [device]`

use std::env;

use minitel::link::{Link, LinkConfig, LinkEvent};
use minitel::protocol;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let mut args = env::args().skip(1);
    let path = match args.next() {
        Some(p) => p,
        None => {
            error!("usage: minitel-show <fichier.vtx> [device]");
            std::process::exit(2);
        }
    };
    let frame = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            error!(%path, error = %e, "lecture .vtx impossible");
            std::process::exit(1);
        }
    };

    let mut cfg = LinkConfig::default();
    if let Some(dev) = args.next() {
        cfg.device = dev.into();
    }
    info!(%path, bytes = frame.len(), device = %cfg.device.display(), "minitel-show");

    let mut link = Link::spawn(cfg);
    let mut sigint = Box::pin(tokio::signal::ctrl_c());
    loop {
        tokio::select! {
            _ = &mut sigint => break,
            evt = link.recv() => match evt {
                Some(LinkEvent::Connected(_)) => {
                    let _ = link.send(protocol::set_echo(false)).await;
                    let _ = link.send(protocol::cursor(false)).await;
                    let _ = link.send(protocol::clear_screen()).await;
                    // nettoie aussi la rangée 0 (non effacée par FF) : y aller + CAN
                    let mut clr0 = protocol::goto_row0();
                    clr0.push(minitel::constants::CANCEL);
                    let _ = link.send(clr0).await;
                    let _ = link.send(frame.clone()).await;
                    info!("image envoyée");
                }
                Some(LinkEvent::Disconnected) => warn!("déconnecté (reconnexion auto)"),
                Some(LinkEvent::Rx(_)) => {} // on ignore le clavier
                None => break,
            }
        }
    }
}
