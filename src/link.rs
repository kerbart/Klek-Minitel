//! Lien série robuste vers le Minitel.
//!
//! Répond au **défaut n°1** des deux audits : le code Python n'avait aucune
//! reconnexion et ses threads d'I/O mouraient en silence sur un glitch USB
//! (fréquent vu l'undervoltage du Pi 3), gelant l'appli sans la crasher.
//!
//! Ici :
//!   - toute erreur d'I/O est un [`Result`] qui remonte (le typage force à
//!     la traiter, contrairement au thread Python qui l'avalait) ;
//!   - une boucle de supervision rouvre le port avec back-off exponentiel
//!     (jusqu'à [`LinkConfig::reconnect_max`]) sur erreur ou EOF série.
//!
//! Config série Minitel : 7 bits, parité paire, 1 stop.

use std::path::PathBuf;
use std::time::Duration;

use serial2_tokio::{CharSize, Parity, SerialPort, Settings, StopBits};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::{info, trace, warn};

use crate::constants::Baud;

#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("erreur d'E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("canal fermé")]
    ChannelClosed,
}

/// Configuration du lien série.
#[derive(Debug, Clone)]
pub struct LinkConfig {
    /// Chemin du port. Préférer le lien stable `by-id` du CH340
    /// (`/dev/serial/by-id/usb-1a86_USB_Serial-if00-port0`).
    pub device: PathBuf,
    /// Vitesse de base (celle du 1B au boot sur la DIN) : 1200 bps.
    pub baud: Baud,
    /// Si vrai, tente de négocier une vitesse plus rapide au démarrage
    /// (voir [`Baud::code`]). Retombe sur [`LinkConfig::baud`] si échec.
    pub fast_baud: Option<Baud>,
    /// Back-off initial de reconnexion.
    pub reconnect_initial: Duration,
    /// Back-off maximal.
    pub reconnect_max: Duration,
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self {
            device: PathBuf::from("/dev/serial/by-id/usb-1a86_USB_Serial-if00-port0"),
            baud: Baud::B1200,
            fast_baud: Some(Baud::B4800), // le 1B monte à 4800
            reconnect_initial: Duration::from_millis(500),
            reconnect_max: Duration::from_secs(15),
        }
    }
}

/// Événement remonté par le lien vers la couche session.
#[derive(Debug, Clone)]
pub enum LinkEvent {
    /// Le port vient d'être (ré)ouvert, à la vitesse indiquée (bauds).
    Connected(u32),
    /// Le port est tombé ; une reconnexion est en cours.
    Disconnected,
    /// Octets reçus du Minitel (clavier / réponses protocole).
    Rx(Vec<u8>),
}

/// Poignée pour piloter le lien : on écrit des octets, on reçoit des events.
pub struct Link {
    tx_out: mpsc::Sender<Vec<u8>>,
    rx_evt: mpsc::Receiver<LinkEvent>,
    ctl: mpsc::Sender<()>, // signal de réinitialisation (referme + renégocie)
}

impl Link {
    /// Démarre la supervision du lien.
    pub fn spawn(cfg: LinkConfig) -> Self {
        let (tx_out, rx_out) = mpsc::channel::<Vec<u8>>(256);
        let (tx_evt, rx_evt) = mpsc::channel::<LinkEvent>(256);
        let (ctl, ctl_rx) = mpsc::channel::<()>(4);
        tokio::spawn(supervise(cfg, rx_out, tx_evt, ctl_rx));
        Self { tx_out, rx_evt, ctl }
    }

    /// File des octets à envoyer au Minitel. Ne bloque pas le port : les
    /// octets sont mis en file et écrits par la tâche de supervision.
    pub async fn send(&self, bytes: Vec<u8>) -> Result<(), LinkError> {
        self.tx_out.send(bytes).await.map_err(|_| LinkError::ChannelClosed)
    }

    /// Prochain événement du lien (connexion, perte, octets reçus).
    pub async fn recv(&mut self) -> Option<LinkEvent> {
        self.rx_evt.recv().await
    }

    /// Force une réinitialisation : referme le port et **renégocie** la vitesse
    /// depuis zéro (utile après un power-cycle du Minitel, où le port reste
    /// ouvert côté Pi mais le terminal est revenu à 1200 bps).
    pub async fn reset(&self) {
        let _ = self.ctl.try_send(());
    }
}

/// Boucle de supervision : (ré)ouvre le port et pompe les octets dans les
/// deux sens tant que la connexion tient. Back-off exponentiel à la perte.
async fn supervise(
    cfg: LinkConfig,
    mut rx_out: mpsc::Receiver<Vec<u8>>,
    tx_evt: mpsc::Sender<LinkEvent>,
    mut ctl_rx: mpsc::Receiver<()>,
) {
    let mut backoff = cfg.reconnect_initial;
    loop {
        match establish(&cfg).await {
            Ok((port, baud)) => {
                info!(device = %cfg.device.display(), baud = baud.rate(), "port prêt");
                backoff = cfg.reconnect_initial; // reset au succès
                if tx_evt.send(LinkEvent::Connected(baud.rate())).await.is_err() {
                    return; // plus personne n'écoute
                }
                if let Err(e) = pump(port, &mut rx_out, &tx_evt, &mut ctl_rx).await {
                    warn!(error = %e, "lien perdu, reconnexion");
                }
                let _ = tx_evt.send(LinkEvent::Disconnected).await;
            }
            Err(e) => {
                warn!(error = %e, backoff_ms = backoff.as_millis(), "ouverture impossible");
            }
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(cfg.reconnect_max);
    }
}

/// Ouvre le port à une vitesse donnée en 7E1 raw (config Minitel).
fn open_at(device: &std::path::Path, baud: Baud) -> Result<SerialPort, LinkError> {
    let rate = baud.rate();
    let port = SerialPort::open(device, move |mut s: Settings| {
        s.set_raw();
        s.set_baud_rate(rate)?;
        s.set_char_size(CharSize::Bits7);
        s.set_stop_bits(StopBits::One);
        s.set_parity(Parity::Even);
        Ok(s)
    })?;
    Ok(port)
}

/// Vérifie qu'un Minitel répond **valablement** à la vitesse courante : envoie
/// PRO1 ENQROM et attend une réponse d'identification `SOH … EOT` sous `timeout`.
///
/// On exige la structure (SOH + EOT) et non « au moins un octet » : à la
/// mauvaise vitesse, les erreurs de framing produisent des octets parasites
/// qui donneraient un faux positif. Consomme la réponse.
async fn probe(port: &mut SerialPort, timeout: Duration) -> bool {
    use crate::constants::{END_OF_TRANSMISSION as EOT, START_OF_HEADING as SOH};
    if port.write_all(&crate::protocol::identify_request()).await.is_err() {
        return false;
    }
    let _ = port.flush().await;
    let got = tokio::time::timeout(timeout, async {
        let mut acc = Vec::new();
        let mut buf = [0u8; 32];
        loop {
            match port.read(&mut buf).await {
                Ok(0) | Err(_) => return false,
                Ok(n) => {
                    acc.extend_from_slice(&buf[..n]);
                    if acc.contains(&SOH) && acc.contains(&EOT) {
                        return true; // réponse d'identification plausible
                    }
                    if acc.len() > 64 {
                        return false; // du bruit, pas une trame protocole
                    }
                }
            }
        }
    })
    .await;
    matches!(got, Ok(true))
}

/// Ouvre le port et négocie la vitesse.
///
/// Le Minitel 1B n'a pas d'EEPROM : il redémarre en 1200 bps sur la DIN. On :
///  1. tente d'ouvrir directement à `fast_baud` et de sonder (cas « déjà rapide »,
///     Minitel resté sous tension) ;
///  2. sinon, ouvre à `baud` (1200), envoie la commande PRO2 de vitesse, puis
///     rebascule l'UART à `fast_baud` et re-sonde ;
///  3. si rien ne répond en rapide, retombe à `baud` (1200), qui marche toujours.
async fn establish(cfg: &LinkConfig) -> Result<(SerialPort, Baud), LinkError> {
    let fast = match cfg.fast_baud {
        None => return Ok((open_at(&cfg.device, cfg.baud)?, cfg.baud)),
        Some(f) => f,
    };

    // 1. déjà en vitesse rapide ?
    let mut port = open_at(&cfg.device, fast)?;
    if probe(&mut port, Duration::from_millis(800)).await {
        return Ok((port, fast));
    }

    // 2. programme la vitesse rapide depuis 1200, avec plusieurs tentatives
    //    (la commutation du 1B est parfois marginale → on réessaie).
    drop(port);
    for attempt in 1..=3 {
        let mut slow = open_at(&cfg.device, cfg.baud)?;
        slow.write_all(&crate::protocol::set_speed(fast)).await?;
        slow.flush().await?;
        tokio::time::sleep(Duration::from_millis(500)).await; // laisse le 1B commuter
        drop(slow);

        let mut port = open_at(&cfg.device, fast)?;
        if probe(&mut port, Duration::from_millis(1200)).await {
            info!(baud = fast.rate(), attempt, "vitesse négociée");
            return Ok((port, fast));
        }
        drop(port);
        warn!(attempt, "négociation rapide ratée, nouvelle tentative");
    }

    // 3. échec après tentatives : on reste à la vitesse de base (fonctionnelle).
    warn!(baud = cfg.baud.rate(), "négociation rapide échouée, repli vitesse de base");
    Ok((open_at(&cfg.device, cfg.baud)?, cfg.baud))
}

/// Pompe bidirectionnelle. Termine (Err ou Ok) dès que le port a un souci,
/// ce qui déclenche une reconnexion en amont.
async fn pump(
    mut port: SerialPort,
    rx_out: &mut mpsc::Receiver<Vec<u8>>,
    tx_evt: &mpsc::Sender<LinkEvent>,
    ctl_rx: &mut mpsc::Receiver<()>,
) -> Result<(), LinkError> {
    let mut buf = [0u8; 256];
    loop {
        tokio::select! {
            // Réinitialisation demandée → referme le port, renégocie en amont.
            _ = ctl_rx.recv() => {
                info!("reset demandé → renégociation");
                return Ok(());
            }
            // Lecture depuis le Minitel.
            read = port.read(&mut buf) => {
                let n = read?;
                if n == 0 {
                    // EOF sur un port série = port disparu (débranché / undervoltage).
                    return Ok(());
                }
                trace!(bytes = n, "rx");
                if tx_evt.send(LinkEvent::Rx(buf[..n].to_vec())).await.is_err() {
                    return Ok(());
                }
            }
            // Écriture vers le Minitel.
            out = rx_out.recv() => {
                match out {
                    Some(bytes) => {
                        port.write_all(&bytes).await?;
                        port.flush().await?;
                        trace!(bytes = bytes.len(), "tx");
                    }
                    None => return Ok(()), // poignée droppée
                }
            }
        }
    }
}
