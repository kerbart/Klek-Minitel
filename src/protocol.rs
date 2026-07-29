//! Couche protocole Videotex : construction des séquences d'octets.
//!
//! Fonctions pures (octets → octets), sans I/O, donc testables sans matériel.
//! La couche session les envoie via [`crate::link::Link`].

use crate::constants::*;

/// Efface tout l'écran (form feed).
pub fn clear_screen() -> Vec<u8> {
    vec![FORM_FEED]
}

/// Efface de la position courante jusqu'à la fin de la ligne.
/// (Corrige le bug Python où `line_end` envoyait `[0x13,0x45]`.)
pub fn clear_line_end() -> Vec<u8> {
    vec![CANCEL]
}

/// Efface la ligne de statut (ligne 0).
pub fn clear_status() -> Vec<u8> {
    vec![UNIT_SEPARATOR, 0x40, 0x41, CANCEL, LINE_FEED]
}

/// Bip.
pub fn beep() -> Vec<u8> {
    vec![BELL]
}

/// Affiche / masque le curseur.
pub fn cursor(visible: bool) -> Vec<u8> {
    vec![if visible { CURSOR_ON } else { CURSOR_OFF }]
}

/// Active/désactive l'écho local clavier→écran (aiguillage PRO3).
///
/// L'init du Minitel **doit** couper l'écho (`set_echo(false)`) sinon chaque
/// touche s'affiche en double dès qu'on gère l'affichage soi-même.
pub fn set_echo(active: bool) -> Vec<u8> {
    let action = if active { ROUTING_ON } else { ROUTING_OFF };
    vec![PROTOCOL_3[0], PROTOCOL_3[1], action, RECV_SCREEN, EMIT_MODEM]
}

/// Active/désactive une **option clavier** (PRO2 `START`/`STOP <option>`).
///
/// Voir [`crate::constants`] : `LOWERCASE`, `EXTENDED`, `CONTROL_0`.
pub fn keyboard_option(option: u8, active: bool) -> Vec<u8> {
    let action = if active { START } else { STOP };
    vec![PROTOCOL_2[0], PROTOCOL_2[1], action, option]
}

/// Passe le clavier en **minuscules** (PRO2 START LOWERCASE).
///
/// Le 1B démarre en mode « majuscules » : sans cette séquence toute la saisie
/// arrive en capitales (et les majuscules accentuées sont inatteignables).
/// À renvoyer **à chaque (re)connexion** — l'option n'est pas mémorisée.
pub fn keyboard_lowercase() -> Vec<u8> {
    keyboard_option(LOWERCASE, true)
}

/// Interroge la ROM du terminal (identification constructeur/type/version).
/// Réponse attendue : `SOH <constructeur> <type> <version> EOT` (5 octets).
pub fn identify_request() -> Vec<u8> {
    vec![PROTOCOL_1[0], PROTOCOL_1[1], ENQUIRY_ROM]
}

/// Interroge le statut de fonctionnement (permet de déduire le mode courant).
pub fn status_operation_request() -> Vec<u8> {
    vec![PROTOCOL_1[0], PROTOCOL_1[1], STATUS_OPERATION]
}

/// Commande PRO2 de programmation de vitesse : `ESC 0x3A PROGRAM <code>`.
/// Le 1B redémarre toujours en 1200 bps sur la prise DIN (pas d'EEPROM) ;
/// on lui envoie cette séquence à 1200, puis on rebascule l'UART hôte à la
/// nouvelle vitesse. Cf. [`crate::constants::Baud::code`].
pub fn set_speed(baud: crate::constants::Baud) -> Vec<u8> {
    vec![PROTOCOL_2[0], PROTOCOL_2[1], PROGRAM, baud.code()]
}

/// Positionne le curseur en (colonne, ligne), base 1, adressage direct US.
/// Videotex : `US <0x40+ligne> <0x40+colonne>`.
pub fn move_to(col: u8, row: u8) -> Vec<u8> {
    vec![UNIT_SEPARATOR, 0x40 + row, 0x40 + col]
}

/// Entre dans la **rangée 0** (ligne de service, au-dessus de l'écran),
/// colonne 1 : `US 0x40 0x41`. Hors zone scrollable → idéal pour un HUD/statut.
pub fn goto_row0() -> Vec<u8> {
    vec![UNIT_SEPARATOR, 0x40, 0x41]
}

/// Encode un texte ASCII imprimable en octets Videotex (best-effort).
/// Les accents (SS2) seront gérés par une table dédiée en Phase 2 ;
/// ici on filtre sur l'ASCII imprimable pour le smoke-test.
pub fn text_ascii(s: &str) -> Vec<u8> {
    s.bytes().filter(|b| (0x20..=0x7E).contains(b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_line_end_is_can_not_the_python_bug() {
        // Régression du bug constants.py:156 : doit être 0x18, pas [0x13,0x45].
        assert_eq!(clear_line_end(), vec![0x18]);
    }

    #[test]
    fn set_echo_off_sequence() {
        assert_eq!(set_echo(false), vec![0x1B, 0x3B, 0x60, 0x58, 0x52]);
    }

    #[test]
    fn keyboard_lowercase_sequence() {
        // PRO2 START LOWERCASE = ESC 0x3A 0x69 0x45
        assert_eq!(keyboard_lowercase(), vec![0x1B, 0x3A, 0x69, 0x45]);
    }

    #[test]
    fn set_speed_4800_sequence() {
        use crate::constants::Baud;
        // PRO2 PROGRAM 4800 = ESC 0x3A 0x6B 0x76
        assert_eq!(set_speed(Baud::B4800), vec![0x1B, 0x3A, 0x6B, 0x76]);
    }

    #[test]
    fn move_to_is_one_based_us_addressing() {
        // colonne 1, ligne 1 -> US @ A  (0x1F 0x41 0x41)
        assert_eq!(move_to(1, 1), vec![0x1F, 0x41, 0x41]);
    }
}
