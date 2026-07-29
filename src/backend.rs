//! Client HTTP du **backend applicatif** (le « cerveau » du terminal).
//!
//! Le Minitel n'a aucun accès réseau : c'est le daemon qui interroge un serveur
//! HTTP que **vous** fournissez, typiquement sur votre LAN. Le contrat est
//! volontairement minuscule — quatre routes, du JSON — pour qu'il soit
//! implémentable en quelques dizaines de lignes dans n'importe quel langage.
//! Voir `AGENTS.md` (« Contrat du backend ») et `examples/backend/`.
//!
//! Un seul aller-retour HTTP/1.0 en clair, volontairement minimal : pas de TLS,
//! pas de `reqwest`/`hyper` — on garde le binaire statique léger. Préférez une
//! **adresse IP** (pas un nom d'hôte) : un binaire musl statique n'a pas de
//! résolveur DNS complet.
//!
//! Le backend renvoie du **texte déjà prêt pour l'écran** (`{ "text": ... }`) :
//! c'est lui qui fait le travail (appel LLM, API, base…) et le met en forme sur
//! 40 colonnes, pas le daemon.

use std::io;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Deserialize)]
struct AskResponse {
    #[serde(default)]
    text: String,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    #[serde(default)]
    net: bool,
}

/// État de la chaîne réseau, tel qu'affiché par le voyant du Minitel.
///
/// Le Minitel lui-même n'a pas d'accès Internet : tout passe par le backend. Il
/// y a donc **deux maillons** qui peuvent casser indépendamment, et le voyant
/// les distingue — c'est la seule façon de savoir, depuis le terminal, s'il faut
/// aller relancer le serveur ou regarder du côté de la connexion Internet.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Net {
    /// Pas encore sondé (au démarrage).
    #[default]
    Unknown,
    /// Backend joignable **et** il atteint Internet → tout marche.
    Online,
    /// Backend joignable mais sans accès Internet → les requêtes qui sortent
    /// échoueront, les services purement locaux marcheront encore.
    NoWeb,
    /// Backend injoignable : serveur éteint, service arrêté, ou lien réseau de
    /// la machine hôte coupé.
    Offline,
}

impl Net {
    /// Libellé pour la barre de statut (rangée 0). **Exactement 6 caractères**
    /// pour tous les états : la barre est cadrée sur 37 colonnes et l'heure est
    /// calée à droite — un libellé de largeur variable ferait sauter l'heure.
    pub fn label(self) -> &'static str {
        match self {
            Net::Unknown => "WEB ??",
            Net::Online => "WEB OK",
            Net::NoWeb => "WEB KO", // le backend répond mais n'atteint pas Internet
            Net::Offline => "SRV KO", // le backend lui-même est injoignable
        }
    }

    /// Message d'erreur à afficher quand une recherche échoue, adapté au maillon
    /// cassé — plus utile qu'un « service indisponible » générique.
    pub fn failure_hint(self) -> &'static str {
        match self {
            Net::Offline => "Serveur injoignable - le verifier.",
            Net::NoWeb => "Le serveur n'a pas d'acces Internet.",
            _ => "Service indisponible.",
        }
    }
}

/// Sonde `GET /health` du backend → état de la chaîne réseau.
///
/// Timeout court et volontairement agressif : c'est un voyant rafraîchi en
/// boucle, pas une requête utilisateur — mieux vaut afficher « SRV KO » une
/// fois de trop que figer l'affichage.
pub async fn health(authority: &str) -> Net {
    let fetch = async {
        let req = format!(
            "GET /health HTTP/1.0\r\nHost: {authority}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
        );
        let mut stream = TcpStream::connect(authority).await?;
        stream.write_all(req.as_bytes()).await?;
        stream.flush().await?;
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await?;
        let body = match find_subslice(&raw, b"\r\n\r\n") {
            Some(i) => &raw[i + 4..],
            None => &raw[..],
        };
        let parsed: HealthResponse = serde_json::from_slice(body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("JSON: {e}")))?;
        io::Result::Ok(parsed.net)
    };
    match tokio::time::timeout(Duration::from_secs(6), fetch).await {
        Ok(Ok(true)) => Net::Online,
        Ok(Ok(false)) => Net::NoWeb,
        // backend muet / erreur JSON / timeout → on ne sait pas le joindre
        _ => Net::Offline,
    }
}

/// Interroge le backend. `authority` = `ip:port` (ex. `192.168.1.10:3009`).
/// `cont` = relance dans le fil de conversation courant (sinon nouvelle convo).
/// Un backend qui appelle un LLM ou fait une recherche web peut mettre plusieurs
/// dizaines de secondes → timeout volontairement large.
pub async fn ask(authority: &str, query: &str, cont: bool) -> io::Result<String> {
    tokio::time::timeout(Duration::from_secs(130), ask_inner(authority, query, cont))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "délai backend dépassé"))?
}

async fn ask_inner(authority: &str, query: &str, cont: bool) -> io::Result<String> {
    let path = format!(
        "/ask?q={}{}",
        urlencode(query),
        if cont { "&cont=1" } else { "" }
    );
    let req = format!(
        "GET {path} HTTP/1.0\r\nHost: {authority}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );

    let mut stream = TcpStream::connect(authority).await?;
    stream.write_all(req.as_bytes()).await?;
    stream.flush().await?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;

    let body = match find_subslice(&raw, b"\r\n\r\n") {
        Some(i) => &raw[i + 4..],
        None => &raw[..],
    };

    let parsed: AskResponse = serde_json::from_slice(body)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("JSON: {e}")))?;
    if let Some(e) = parsed.error {
        return Err(io::Error::new(io::ErrorKind::Other, e));
    }
    Ok(parsed.text)
}

/// Récupère un service du menu Guide : `GET /service?name=<nom>`.
///
/// Les noms sont libres — ce sont ceux que vous déclarez dans `services.json`
/// côté daemon et que votre backend sait servir.
pub async fn service(authority: &str, name: &str) -> io::Result<String> {
    let query_path = format!("/service?name={}", urlencode(name));
    tokio::time::timeout(Duration::from_secs(40), async {
        let req = format!(
            "GET {query_path} HTTP/1.0\r\nHost: {authority}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
        );
        let mut stream = TcpStream::connect(authority).await?;
        stream.write_all(req.as_bytes()).await?;
        stream.flush().await?;
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await?;
        let body = match find_subslice(&raw, b"\r\n\r\n") {
            Some(i) => &raw[i + 4..],
            None => &raw[..],
        };
        let parsed: AskResponse = serde_json::from_slice(body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("JSON: {e}")))?;
        if let Some(e) = parsed.error {
            return Err(io::Error::new(io::ErrorKind::Other, e));
        }
        io::Result::Ok(parsed.text)
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "délai service dépassé"))?
}

/// Réinitialise le fil de conversation côté backend (Sommaire). Best-effort.
pub async fn reset(authority: &str) {
    let _ = tokio::time::timeout(Duration::from_secs(5), async {
        let req = format!("GET /reset HTTP/1.0\r\nHost: {authority}\r\nConnection: close\r\n\r\n");
        let mut stream = TcpStream::connect(authority).await?;
        stream.write_all(req.as_bytes()).await?;
        stream.flush().await?;
        let mut sink = Vec::new();
        stream.read_to_end(&mut sink).await?;
        io::Result::Ok(())
    })
    .await;
}

/// Percent-encoding minimal (les caractères non sûrs → %XX).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_space_and_accents() {
        assert_eq!(urlencode("meteo a paris"), "meteo+a+paris");
        assert_eq!(urlencode("café"), "caf%C3%A9"); // é en UTF-8
    }

    #[test]
    fn net_labels_all_six_chars() {
        // invariant de cadrage : l'heure de la rangée 0 est calée à droite, un
        // libellé plus court ou plus long la ferait sauter
        for n in [Net::Unknown, Net::Online, Net::NoWeb, Net::Offline] {
            assert_eq!(n.label().chars().count(), 6, "label {n:?}");
        }
    }

    #[test]
    fn health_json_maps_to_states() {
        let online: HealthResponse = serde_json::from_slice(br#"{"ok":true,"net":true}"#).unwrap();
        assert!(online.net);
        let noweb: HealthResponse = serde_json::from_slice(br#"{"ok":true,"net":false}"#).unwrap();
        assert!(!noweb.net);
        // backend d'une version antérieure : pas de champ `net` → prudent (false)
        let legacy: HealthResponse = serde_json::from_slice(br#"{"ok":true}"#).unwrap();
        assert!(!legacy.net);
    }

    #[test]
    fn parse_response_json() {
        let body = br#"{"query":"x","text":"ZAGREB\nCapitale de la Croatie."}"#;
        let r: AskResponse = serde_json::from_slice(body).unwrap();
        assert!(r.text.starts_with("ZAGREB"));
        assert!(r.error.is_none());
    }
}
