//! Codec Videotex — encodage vers l'écran du Minitel.
//!
//! Fonctions pures (→ `Vec<u8>`), sans I/O : table d'accents Unicode → G2 et
//! attributs couleur/taille.
//!
//! Complète [`crate::protocol`] (commandes protocole) côté **affichage** :
//! texte accentué, couleurs, attributs.

use crate::constants::*;

/// Séquence SS2 (accent / caractère spécial) pour un `char`.
/// Retourne `None` si le caractère n'a pas d'équivalent Videotex dédié.
fn ss2_sequence(c: char) -> Option<Vec<u8>> {
    let ss2 = SINGLE_SHIFT_2;
    // Diacritique : grave=0x41, aigu=0x42, circonflexe=0x43, tréma=0x48 ; puis lettre.
    let acc = |diac: u8, letter: u8| Some(vec![ss2, diac, letter]);
    Some(match c {
        // lettres accentuées courantes
        'à' => return acc(0x41, b'a'), 'á' => return acc(0x42, b'a'),
        'â' => return acc(0x43, b'a'), 'ä' => return acc(0x48, b'a'),
        'è' => return acc(0x41, b'e'), 'é' => return acc(0x42, b'e'),
        'ê' => return acc(0x43, b'e'), 'ë' => return acc(0x48, b'e'),
        'ì' => return acc(0x41, b'i'), 'í' => return acc(0x42, b'i'),
        'î' => return acc(0x43, b'i'), 'ï' => return acc(0x48, b'i'),
        'ò' => return acc(0x41, b'o'), 'ó' => return acc(0x42, b'o'),
        'ô' => return acc(0x43, b'o'), 'ö' => return acc(0x48, b'o'),
        'ù' => return acc(0x41, b'u'), 'ú' => return acc(0x42, b'u'),
        'û' => return acc(0x43, b'u'), 'ü' => return acc(0x48, b'u'),
        'ç' => return Some(vec![ss2, 0x4B, b'c']),
        // majuscules accentuées (même mécanisme SS2 + lettre de base)
        'À' => return acc(0x41, b'A'), 'Â' => return acc(0x43, b'A'), 'Ä' => return acc(0x48, b'A'),
        'É' => return acc(0x42, b'E'), 'È' => return acc(0x41, b'E'),
        'Ê' => return acc(0x43, b'E'), 'Ë' => return acc(0x48, b'E'),
        'Î' => return acc(0x43, b'I'), 'Ï' => return acc(0x48, b'I'),
        'Ô' => return acc(0x43, b'O'), 'Ö' => return acc(0x48, b'O'),
        'Û' => return acc(0x43, b'U'), 'Ü' => return acc(0x48, b'U'), 'Ù' => return acc(0x41, b'U'),
        'Ç' => return Some(vec![ss2, 0x4B, b'C']),
        '\u{2019}' => return Some(vec![ss2, 0x4B, 0x27]), // apostrophe typographique
        // caractères spéciaux (1 octet après SS2)
        '£' => vec![ss2, 0x23], '°' => vec![ss2, 0x30], '±' => vec![ss2, 0x31],
        '←' => vec![ss2, 0x2C], '↑' => vec![ss2, 0x2D],
        '→' => vec![ss2, 0x2E], '↓' => vec![ss2, 0x2F],
        '¼' => vec![ss2, 0x3C], '½' => vec![ss2, 0x3D], '¾' => vec![ss2, 0x3E],
        'Œ' => vec![ss2, 0x6A], 'œ' => vec![ss2, 0x7A], 'ß' => vec![ss2, 0x7B],
        _ => return None,
    })
}

/// Encode une chaîne Unicode en octets Videotex (accents gérés).
/// Les caractères sans équivalent sont ignorés (best-effort, jamais de panique).
pub fn encode_text(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        if (0x20..=0x7E).contains(&(c as u32)) {
            out.push(c as u8); // ASCII imprimable direct
        } else if let Some(seq) = ss2_sequence(c) {
            out.extend(seq);
        }
        // sinon : caractère non représentable → ignoré
    }
    out
}

/// Attribut : couleur du texte (ESC 0x40+code).
pub fn fg(color: Color) -> Vec<u8> {
    vec![ESCAPE, ATTR_FG_BASE + color.code()]
}

/// Attribut : couleur de fond (ESC 0x50+code).
pub fn bg(color: Color) -> Vec<u8> {
    vec![ESCAPE, ATTR_BG_BASE + color.code()]
}

/// Texte coloré prêt à envoyer : `fg(color)` + texte encodé.
pub fn colored(color: Color, s: &str) -> Vec<u8> {
    let mut v = fg(color);
    v.extend(encode_text(s));
    v
}

/// Taille des caractères (attribut ESC 0x4C..0x4F).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Normal,
    DoubleHeight,
    DoubleWidth,
    DoubleSize,
}

/// Attribut de taille. ⚠️ double hauteur/taille interdite en ligne 1 et rangée 0,
/// et interdite en mosaïque (G1).
pub fn size(s: Size) -> Vec<u8> {
    let code = match s {
        Size::Normal => 0x4C,
        Size::DoubleHeight => 0x4D,
        Size::DoubleWidth => 0x4E,
        Size::DoubleSize => 0x4F,
    };
    vec![ESCAPE, code]
}

/// Clignotement on/off.
pub fn blink(on: bool) -> Vec<u8> {
    vec![ESCAPE, if on { 0x48 } else { 0x49 }]
}

/// Inversion vidéo on/off (attribut de zone : prévoir un espace délimiteur).
pub fn invert(on: bool) -> Vec<u8> {
    vec![ESCAPE, if on { 0x5D } else { 0x5C }]
}

/// Soulignement on/off (zone ; en G1 : mosaïque disjointe/jointive).
pub fn underline(on: bool) -> Vec<u8> {
    vec![ESCAPE, if on { 0x5A } else { 0x59 }]
}

/// Bascule vers le jeu **G1** (mosaïque semi-graphique).
pub fn g1() -> u8 {
    SHIFT_OUT // 0x0E
}
/// Retour au jeu **G0** (alphanumérique).
pub fn g0() -> u8 {
    SHIFT_IN // 0x0F
}

/// Encode un motif mosaïque 2×3 (6 bits) en octet G1 affichable.
///
/// Bits : b0 haut-g, b1 haut-d, b2 milieu-g, b3 milieu-d, b4 bas-g, b5 bas-d.
/// Formule STUM : `0x20 + (p & 0x1F) + (si b5 : 0x40)`.
pub fn mosaic(pattern: u8) -> u8 {
    0x20 + (pattern & 0x1F) + if pattern & 0x20 != 0 { 0x40 } else { 0 }
}

/// Répète le **dernier caractère envoyé** `n` fois (`n` ≤ 63).
/// À émettre juste après le caractère à répéter.
pub fn rep(n: u8) -> Vec<u8> {
    vec![REPEAT, 0x40 + n.min(63)]
}

/// Découpe un texte en lignes d'au plus `width` colonnes, sans couper les mots
/// (sauf mot plus long que `width`). Utile pour tenir dans les 40 colonnes du
/// Minitel. Compte en `char` (un accent = 1 colonne à l'écran).
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if word.chars().count() > width {
            // mot trop long : on le coupe brutalement
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            let mut chunk = String::new();
            for c in word.chars() {
                if chunk.chars().count() == width {
                    lines.push(std::mem::take(&mut chunk));
                }
                chunk.push(c);
            }
            cur = chunk;
            continue;
        }
        let sep = if cur.is_empty() { 0 } else { 1 };
        if cur.chars().count() + sep + word.chars().count() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_respects_width_and_words() {
        let w = wrap("le minitel est un terminal videotex francais", 20);
        assert!(w.iter().all(|l| l.chars().count() <= 20));
        assert_eq!(w.join(" "), "le minitel est un terminal videotex francais");
    }

    #[test]
    fn wrap_breaks_overlong_word() {
        let w = wrap("aaaaaaaaaaaaaaaaaaaaaaaaa", 10); // 25 'a'
        assert_eq!(w.len(), 3);
        assert!(w.iter().all(|l| l.chars().count() <= 10));
    }

    #[test]
    fn ascii_passthrough() {
        assert_eq!(encode_text("AB1"), vec![b'A', b'B', b'1']);
    }

    #[test]
    fn e_aigu_roundtrip_with_decoder() {
        // encode(é) doit être décodable par input::Decoder en Char('é').
        let bytes = encode_text("é");
        assert_eq!(bytes, vec![0x19, 0x42, b'e']);
        let mut d = crate::input::Decoder::new();
        assert_eq!(d.push(&bytes), vec![crate::input::Event::Char('é')]);
    }

    #[test]
    fn cedilla_and_accents_in_word() {
        // "café" = c a f é
        assert_eq!(
            encode_text("café"),
            vec![b'c', b'a', b'f', 0x19, 0x42, b'e']
        );
    }

    #[test]
    fn mosaic_encoding() {
        assert_eq!(mosaic(0x00), 0x20); // tous éteints = espace
        assert_eq!(mosaic(0x3F), 0x7F); // tous allumés
        assert_eq!(mosaic(0x20), 0x60); // seul le bloc bas-droit (bit5)
    }

    #[test]
    fn rep_sequence() {
        assert_eq!(rep(5), vec![0x12, 0x45]);
        assert_eq!(rep(200), vec![0x12, 0x40 + 63]); // borné à 63
    }

    #[test]
    fn fg_color_sequence() {
        // texte rouge : ESC 0x41
        assert_eq!(fg(Color::Red), vec![0x1B, 0x41]);
        // fond bleu : ESC 0x54 (0x50 + 4)
        assert_eq!(bg(Color::Blue), vec![0x1B, 0x54]);
    }
}
