//! Pilote Minitel — bibliothèque.
//!
//! Parler à un vrai terminal Minitel (1B et compatibles) depuis Rust, via un
//! simple adaptateur USB-UART branché sur la prise péri-informatique DIN.
//!
//! ```text
//!   [link]      lien série robuste (négociation de vitesse, reconnexion)
//!     ↓
//!   [protocol]  séquences Vidéotex brutes (curseur, couleurs, rangée 0…)
//!   [videotex]  encodage de texte (accents, césure, mosaïque G1)
//!   [constants] codes du protocole (C0, G0/G1/G2, touches de fonction)
//!     ↓
//!   [input]     décodage du clavier (touches, flèches, accusés PRO)
//!   [edit]      éditeur de saisie multi-lignes projeté sur la grille 40×24
//!     ↓
//!   [backend]   client HTTP minimal vers *votre* serveur (facultatif)
//! ```
//!
//! Les modules `link` → `edit` n'ont **aucune** dépendance à un service
//! particulier : ils forment le pilote réutilisable. [`backend`] n'est qu'un
//! client HTTP/1.0 sans TLS utilisé par le daemon `miniteld` fourni en
//! exemple — ignorez-le si vous écrivez votre propre application.

pub mod backend;
pub mod constants;
pub mod edit;
pub mod input;
pub mod link;
pub mod protocol;
pub mod videotex;
