//! Constantes du protocole Minitel / Videotex.
//!
//! Source de vérité pour les séquences d'échappement du Minitel 1B. Les valeurs
//! reprennent la lignée des implémentations Python (PyMinitel et dérivés),
//! revérifiées sur matériel.
//!
//! ⚠️ Piège hérité de ces implémentations : `CANCEL` y était défini à `0x18` puis
//! **réassigné** à `[0x13, 0x45]`, ce qui cassait `clear('line_end')`. Ici on
//! distingue proprement :
//!   - [`CANCEL`] = `0x18` (le vrai caractère de contrôle C0 « CAN »),
//!   - [`CLEAR_LINE_END`] = `[0x18]` (efface jusqu'à la fin de ligne),
//!   - [`CLEAR_STATUS`] pour la ligne de statut.

// --- Caractères de contrôle C0 ---
pub const NUL: u8 = 0x00;
pub const START_OF_HEADING: u8 = 0x01; // SOH
pub const END_OF_TRANSMISSION: u8 = 0x04; // EOT
pub const ENQUIRY: u8 = 0x05; // ENQ
pub const BELL: u8 = 0x07; // BEL — bip
pub const BACKSPACE: u8 = 0x08; // BS
pub const TAB: u8 = 0x09; // HT
pub const LINE_FEED: u8 = 0x0A; // LF
pub const VERTICAL_TAB: u8 = 0x0B; // VT
pub const FORM_FEED: u8 = 0x0C; // FF — efface tout l'écran
pub const CARRIAGE_RETURN: u8 = 0x0D; // CR
pub const SHIFT_OUT: u8 = 0x0E; // SO — bascule jeu semi-graphique
pub const SHIFT_IN: u8 = 0x0F; // SI — bascule jeu alphanumérique
pub const CURSOR_ON: u8 = 0x11; // DC1 — affiche le curseur
pub const REPEAT: u8 = 0x12; // DC2 — répétition de caractère
pub const DEVICE_CONTROL_3: u8 = 0x13; // DC3 (= SEP protocole)
pub const CURSOR_OFF: u8 = 0x14; // DC4 — masque le curseur
pub const CANCEL: u8 = 0x18; // CAN — efface jusqu'à la fin de ligne
pub const SINGLE_SHIFT_2: u8 = 0x19; // SS2 — préfixe caractères accentués
pub const ESCAPE: u8 = 0x1B; // ESC
pub const RECORD_SEPARATOR: u8 = 0x1E; // RS
pub const UNIT_SEPARATOR: u8 = 0x1F; // US — adressage ligne de statut

// Alias : DC3 sert aussi de séparateur dans certaines séquences protocole.
pub const SEPARATOR: u8 = DEVICE_CONTROL_3;

// --- Préfixes de séquences (multi-octets) ---
pub const CSI: [u8; 2] = [ESCAPE, 0x5B]; // Control Sequence Introducer
pub const PROTOCOL_1: [u8; 2] = [ESCAPE, 0x39]; // PRO1
pub const PROTOCOL_2: [u8; 2] = [ESCAPE, 0x3A]; // PRO2
pub const PROTOCOL_3: [u8; 2] = [ESCAPE, 0x3B]; // PRO3

// --- Commandes protocole ---
pub const ENQUIRY_ROM: u8 = 0x7B; // interroge la ROM (identification)
pub const STATUS_TERMINAL: u8 = 0x70;
pub const STATUS_OPERATION: u8 = 0x72;
pub const START: u8 = 0x69; // active une option
pub const STOP: u8 = 0x6A; // désactive une option
pub const PROGRAM: u8 = 0x6B; // programme (ex. vitesse)

// Aiguillage (routing) — utilisé pour l'écho clavier→écran
pub const ROUTING_OFF: u8 = 0x60;
pub const ROUTING_ON: u8 = 0x61;
pub const RECV_SCREEN: u8 = 0x58; // récepteur = écran
pub const RECV_KEYBOARD: u8 = 0x59; // récepteur = clavier
pub const EMIT_MODEM: u8 = 0x52; // émetteur = modem

// Options clavier
pub const EXTENDED: u8 = 0x41; // clavier étendu
pub const CONTROL_0: u8 = 0x43; // touches curseur
pub const LOWERCASE: u8 = 0x45; // minuscules

// --- Longueurs de réponse attendues (acquittements protocole) ---
pub const PROTOCOL_1_LENGTH: usize = 3;
pub const PROTOCOL_2_LENGTH: usize = 4;
pub const PROTOCOL_3_LENGTH: usize = 5;

/// Vitesses supportées et leur code de programmation (commande PRO2 PROGRAM).
///
/// ⚠️ Un Minitel **1B plafonne à 4800 bps**. Ne jamais tester 9600 en premier
/// (bug du Python `detect_speed` qui commençait par 9600).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Baud {
    B300,
    B1200,
    B4800,
    B9600,
}

impl Baud {
    /// Code de programmation vitesse (PRO2 PROGRAM <code>).
    pub const fn code(self) -> u8 {
        match self {
            Baud::B300 => 0x52,
            Baud::B1200 => 0x64,
            Baud::B4800 => 0x76,
            Baud::B9600 => 0x7F,
        }
    }

    /// Débit en bauds pour la config du port série.
    pub const fn rate(self) -> u32 {
        match self {
            Baud::B300 => 300,
            Baud::B1200 => 1200,
            Baud::B4800 => 4800,
            Baud::B9600 => 9600,
        }
    }

    /// Ordre de détection sûr pour un 1B : du plus courant au plus rare,
    /// sans 9600 (hors specs du 1B). Le 1B démarre à 1200 par défaut.
    pub const fn detection_order() -> [Baud; 3] {
        [Baud::B1200, Baud::B4800, Baud::B300]
    }
}

// --- Touches de fonction (émises par le Minitel : SEP=DC3 puis code) ---
// Le clavier envoie `0x13 <code>` pour les touches de fonction.
pub const FKEY_SEND: u8 = 0x41; // Envoi
pub const FKEY_RETURN: u8 = 0x42; // Retour
pub const FKEY_REPEAT: u8 = 0x43; // Répétition
pub const FKEY_GUIDE: u8 = 0x44; // Guide
pub const FKEY_CANCEL: u8 = 0x45; // Annulation
pub const FKEY_SUMMARY: u8 = 0x46; // Sommaire
pub const FKEY_CORRECTION: u8 = 0x47; // Correction
pub const FKEY_NEXT: u8 = 0x48; // Suite
pub const FKEY_CONNECT: u8 = 0x49; // Connexion/Fin

// --- Couleurs Videotex (palette ordonnée par luminance croissante) ---
// Nommage sémantique → code 0..7 utilisé dans les attributs Videotex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
}

impl Color {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

// Attributs Videotex : préfixe ESC puis un octet dans 0x40..0x5F.
// Couleur de texte  = ESC (0x40 + code couleur).
// Couleur de fond   = ESC (0x50 + code couleur).
pub const ATTR_FG_BASE: u8 = 0x40;
pub const ATTR_BG_BASE: u8 = 0x50;
