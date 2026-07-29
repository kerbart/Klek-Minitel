//! Décodeur du flux entrant du Minitel.
//!
//! Répond au point laissé ouvert par le smoke-test Phase 1 : les octets reçus
//! ne sont pas tous des touches — le Minitel renvoie aussi des **accusés
//! protocole** (réponses PRO2/PRO3), sa réponse d'**identification**
//! (`SOH … EOT`), des **touches de fonction** (`SEP=0x13 <code>`), des
//! **flèches** (`CSI …`) et des **caractères accentués** (`SS2=0x19 <acc> <lettre>`).
//!
//! [`Decoder`] est une machine à états : on lui pousse les octets bruts
//! ([`Decoder::push`]) et elle émet des [`Event`] complets. Elle ne fait
//! aucune I/O → testable sans matériel.

use crate::constants::*;

/// Touche de fonction du Minitel (rangée de touches sous l'écran).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnKey {
    Send,       // Envoi
    Return,     // Retour
    Repeat,     // Répétition
    Guide,      // Guide
    Cancel,     // Annulation
    Summary,    // Sommaire
    Correction, // Correction
    Next,       // Suite
    Connect,    // Connexion/Fin
}

impl FnKey {
    fn from_code(code: u8) -> Option<FnKey> {
        Some(match code {
            FKEY_SEND => FnKey::Send,
            FKEY_RETURN => FnKey::Return,
            FKEY_REPEAT => FnKey::Repeat,
            FKEY_GUIDE => FnKey::Guide,
            FKEY_CANCEL => FnKey::Cancel,
            FKEY_SUMMARY => FnKey::Summary,
            FKEY_CORRECTION => FnKey::Correction,
            FKEY_NEXT => FnKey::Next,
            FKEY_CONNECT => FnKey::Connect,
            _ => return None,
        })
    }
}

/// Direction d'une touche flèche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    Up,
    Down,
    Left,
    Right,
}

/// Événement décodé depuis le Minitel.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Caractère imprimable saisi (déjà décodé, accents compris).
    Char(char),
    /// Touche Entrée (CR).
    Enter,
    /// Touche de fonction.
    Function(FnKey),
    /// Flèche du curseur.
    Arrow(Arrow),
    /// Accusé / réponse protocole (séquence brute commençant par ESC).
    /// À interpréter par la couche protocole, PAS par l'UI.
    ProtocolAck(Vec<u8>),
    /// Réponse d'identification ROM : `SOH <constructeur> <type> <version> EOT`.
    Identify { constructor: u8, device: u8, version: u8 },
    /// Octet non reconnu (remonté pour ne rien perdre silencieusement).
    Unknown(u8),
}

/// État interne de la machine à états.
#[derive(Debug, Clone, PartialEq)]
enum State {
    Ground,
    /// Après ESC : on accumule une séquence protocole/CSI.
    Escape(Vec<u8>),
    /// Après SEP (0x13) : on attend le code de touche de fonction.
    Sep,
    /// Après SS2 (0x19) : accent en cours, octets accumulés.
    Accent(Vec<u8>),
    /// Après SOH (0x01) : réponse d'identification jusqu'à EOT.
    Identify(Vec<u8>),
}

/// Décodeur incrémental du flux Minitel → [`Event`].
#[derive(Debug)]
pub struct Decoder {
    state: State,
}

impl Default for Decoder {
    fn default() -> Self {
        Self { state: State::Ground }
    }
}

impl Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pousse un lot d'octets, retourne les événements complets décodés.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Event> {
        let mut out = Vec::new();
        for &b in bytes {
            self.step(b, &mut out);
        }
        out
    }

    fn step(&mut self, b: u8, out: &mut Vec<Event>) {
        match std::mem::replace(&mut self.state, State::Ground) {
            State::Ground => self.ground(b, out),
            State::Escape(mut acc) => {
                acc.push(b);
                // Fin d'une séquence protocole/CSI :
                //  - CSI (ESC [) : se termine sur une lettre finale (0x40..0x7E),
                //  - PRO2/PRO3 (ESC 0x3A/0x3B …) : longueur fixe connue.
                if seq_complete(&acc) {
                    // Les flèches sont des CSI : on les reconnaît comme touches ;
                    // le reste (accusés PRO2/3, CSI divers) part en ProtocolAck.
                    match arrow_from_csi(&acc) {
                        Some(a) => out.push(Event::Arrow(a)),
                        None => out.push(Event::ProtocolAck(acc)),
                    }
                    self.state = State::Ground;
                } else {
                    self.state = State::Escape(acc);
                }
            }
            State::Sep => {
                match FnKey::from_code(b) {
                    Some(k) => out.push(Event::Function(k)),
                    None => out.push(Event::Unknown(b)),
                }
                self.state = State::Ground;
            }
            State::Accent(mut acc) => {
                acc.push(b);
                // SS2 suivi d'un diacritique (0x41..0x4B) puis de la lettre.
                // Un accent « seul » (£, °, flèches…) tient sur 1 octet après SS2.
                if acc.len() == 1 && !(0x41..=0x4B).contains(&acc[0]) {
                    // caractère spécial direct (ex. 0x23=£, 0x30=°)
                    out.push(decode_ss2(&acc));
                    self.state = State::Ground;
                } else if acc.len() >= 2 {
                    out.push(decode_ss2(&acc));
                    self.state = State::Ground;
                } else {
                    self.state = State::Accent(acc);
                }
            }
            State::Identify(mut acc) => {
                if b == END_OF_TRANSMISSION {
                    // acc = [constructeur, type, version]
                    if acc.len() == 3 {
                        out.push(Event::Identify {
                            constructor: acc[0],
                            device: acc[1],
                            version: acc[2],
                        });
                    } else {
                        out.push(Event::ProtocolAck({
                            let mut v = vec![START_OF_HEADING];
                            v.extend_from_slice(&acc);
                            v.push(END_OF_TRANSMISSION);
                            v
                        }));
                    }
                    self.state = State::Ground;
                } else {
                    acc.push(b);
                    self.state = State::Identify(acc);
                }
            }
        }
    }

    fn ground(&mut self, b: u8, out: &mut Vec<Event>) {
        match b {
            ESCAPE => self.state = State::Escape(vec![]),
            DEVICE_CONTROL_3 => self.state = State::Sep, // 0x13 = SEP touche fonction
            SINGLE_SHIFT_2 => self.state = State::Accent(vec![]),
            START_OF_HEADING => self.state = State::Identify(vec![]),
            CARRIAGE_RETURN => out.push(Event::Enter),
            0x20..=0x7E => out.push(Event::Char(b as char)),
            _ => out.push(Event::Unknown(b)),
        }
    }
}

/// Une séquence ESC-préfixée est-elle complète ?
fn seq_complete(acc: &[u8]) -> bool {
    match acc.first() {
        // CSI : ESC [ … <lettre finale 0x40..0x7E>
        Some(&0x5B) => acc.len() >= 2 && (0x40..=0x7E).contains(acc.last().unwrap()),
        // PRO1 (ESC 0x39 <cmd>) → 3 octets au total avec ESC ; ici acc commence après ESC
        Some(&0x39) => acc.len() >= 2,
        // PRO2 (ESC 0x3A <cmd> <param>)
        Some(&0x3A) => acc.len() >= 3,
        // PRO3 (ESC 0x3B <cmd> <p1> <p2>)
        Some(&0x3B) => acc.len() >= 4,
        // Attribut isolé (couleur…) : 1 octet après ESC
        Some(&x) if (0x40..=0x5F).contains(&x) => true,
        _ => acc.len() >= 1,
    }
}

/// Reconnaît une flèche parmi une séquence CSI complète (`[0x5B, dir]`).
fn arrow_from_csi(acc: &[u8]) -> Option<Arrow> {
    if acc.len() == 2 && acc[0] == 0x5B {
        return Some(match acc[1] {
            0x41 => Arrow::Up,
            0x42 => Arrow::Down,
            0x44 => Arrow::Left,
            0x43 => Arrow::Right,
            _ => return None,
        });
    }
    None
}

/// Décode une séquence SS2 (accent) vers un `char` Unicode.
/// Réciproque de la table `videotex::UNICODE_TO_VIDEOTEX`.
fn decode_ss2(acc: &[u8]) -> Event {
    // Diacritique + lettre : [0x41=grave,0x42=aigu,0x43=circonflexe,0x48=tréma] + lettre.
    if acc.len() == 2 {
        let (diac, letter) = (acc[0], acc[1] as char);
        let c = match (diac, letter) {
            (0x41, 'a') => 'à', (0x42, 'a') => 'á', (0x43, 'a') => 'â', (0x48, 'a') => 'ä',
            (0x41, 'e') => 'è', (0x42, 'e') => 'é', (0x43, 'e') => 'ê', (0x48, 'e') => 'ë',
            (0x41, 'i') => 'ì', (0x42, 'i') => 'í', (0x43, 'i') => 'î', (0x48, 'i') => 'ï',
            (0x41, 'o') => 'ò', (0x42, 'o') => 'ó', (0x43, 'o') => 'ô', (0x48, 'o') => 'ö',
            (0x41, 'u') => 'ù', (0x42, 'u') => 'ú', (0x43, 'u') => 'û', (0x48, 'u') => 'ü',
            (0x4B, 'c') => 'ç',
            _ => return Event::Unknown(acc[1]),
        };
        return Event::Char(c);
    }
    if acc.len() == 1 {
        let c = match acc[0] {
            0x23 => '£', 0x30 => '°', 0x31 => '±',
            0x2C => '←', 0x2D => '↑', 0x2E => '→', 0x2F => '↓',
            0x6A => 'Œ', 0x7A => 'œ', 0x7B => 'ß',
            other => return Event::Unknown(other),
        };
        return Event::Char(c);
    }
    Event::Unknown(*acc.last().unwrap_or(&0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        let mut d = Decoder::new();
        assert_eq!(
            d.push(b"Hi"),
            vec![Event::Char('H'), Event::Char('i')]
        );
    }

    #[test]
    fn enter_key() {
        let mut d = Decoder::new();
        assert_eq!(d.push(&[0x0D]), vec![Event::Enter]);
    }

    #[test]
    fn function_key_envoi() {
        // SEP (0x13) + 0x41 = touche Envoi
        let mut d = Decoder::new();
        assert_eq!(d.push(&[0x13, 0x41]), vec![Event::Function(FnKey::Send)]);
    }

    #[test]
    fn function_key_sommaire_split_across_pushes() {
        // le décodeur doit tenir l'état entre deux lots
        let mut d = Decoder::new();
        assert_eq!(d.push(&[0x13]), vec![]);
        assert_eq!(d.push(&[0x46]), vec![Event::Function(FnKey::Summary)]);
    }

    #[test]
    fn accented_e_aigu() {
        // SS2 (0x19) + 0x42 (aigu) + 'e'  => é
        let mut d = Decoder::new();
        assert_eq!(d.push(&[0x19, 0x42, b'e']), vec![Event::Char('é')]);
    }

    #[test]
    fn cedilla() {
        let mut d = Decoder::new();
        assert_eq!(d.push(&[0x19, 0x4B, b'c']), vec![Event::Char('ç')]);
    }

    #[test]
    fn protocol_ack_pro3_not_seen_as_keys() {
        // Réponse PRO3 = 5 octets (ESC 0x3B <cmd> <p1> <p2>). NE doit PAS
        // être décodée comme des touches ; un octet 0x58/0x52 isolé serait
        // sinon pris pour un caractère.
        let mut d = Decoder::new();
        let ev = d.push(&[0x1B, 0x3B, 0x60, 0x58, 0x52]);
        assert_eq!(ev, vec![Event::ProtocolAck(vec![0x3B, 0x60, 0x58, 0x52])]);
    }

    #[test]
    fn identify_response() {
        // SOH 'B' 'v' '4' EOT
        let mut d = Decoder::new();
        let ev = d.push(&[0x01, b'B', b'v', b'4', 0x04]);
        assert_eq!(
            ev,
            vec![Event::Identify { constructor: b'B', device: b'v', version: b'4' }]
        );
    }

    #[test]
    fn arrow_up_decoded() {
        // Flèche haut = CSI (ESC [) 'A' → Event::Arrow(Up).
        let mut d = Decoder::new();
        assert_eq!(d.push(&[0x1B, 0x5B, 0x41]), vec![Event::Arrow(Arrow::Up)]);
    }
}
