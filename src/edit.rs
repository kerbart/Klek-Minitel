//! Éditeur de saisie multi-lignes pour le Minitel.
//!
//! Un buffer de `char` + une position de curseur, projeté sur une zone
//! rectangulaire de `cols`×`rows` cases. Retour à la ligne **par largeur fixe**
//! (chaque `cols` caractères) : simple et prévisible pour mapper le curseur.
//!
//! Gère : insertion, effacement (Correction), flèches (← → ↑ ↓), Annulation.
//! Ne fait aucune I/O — la couche daemon dessine le résultat.

pub struct Editor {
    buf: Vec<char>,
    cursor: usize, // index dans buf (0..=len)
    cols: usize,
    rows: usize,
}

impl Editor {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { buf: Vec::new(), cursor: 0, cols, rows }
    }

    pub fn capacity(&self) -> usize {
        self.cols * self.rows
    }

    /// Change la hauteur de la zone de saisie.
    ///
    /// Le champ n'a pas la même taille selon l'écran : 4 lignes dans la zone de
    /// recherche de l'accueil, 2 lignes seulement quand il est collé en bas en
    /// mode chat. Sans ce réglage, la capacité resterait celle de l'accueil et
    /// une saisie longue déborderait **sous** la dernière ligne de l'écran —
    /// ce qui fait défiler tout l'affichage sur un Minitel.
    pub fn set_rows(&mut self, rows: usize) {
        self.rows = rows.max(1);
        self.buf.truncate(self.capacity());
        self.cursor = self.cursor.min(self.buf.len());
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Le curseur est-il en fin de buffer ? (cas « ajout » = écho direct rapide)
    pub fn at_end(&self) -> bool {
        self.cursor == self.buf.len()
    }

    /// Colonne courante du curseur (0-based).
    pub fn cursor_col(&self) -> usize {
        self.cursor % self.cols
    }

    pub fn text(&self) -> String {
        self.buf.iter().collect()
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.cursor = 0;
    }

    /// Insère un caractère au curseur. `false` si la zone est pleine.
    pub fn insert(&mut self, c: char) -> bool {
        if self.buf.len() >= self.capacity() {
            return false;
        }
        self.buf.insert(self.cursor, c);
        self.cursor += 1;
        true
    }

    /// Efface le caractère **avant** le curseur (Correction). `false` si rien.
    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.buf.remove(self.cursor);
        true
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    pub fn right(&mut self) {
        if self.cursor < self.buf.len() {
            self.cursor += 1;
        }
    }
    pub fn up(&mut self) {
        self.cursor = self.cursor.saturating_sub(self.cols);
    }
    pub fn down(&mut self) {
        let n = (self.cursor + self.cols).min(self.buf.len());
        self.cursor = n;
    }

    /// Position (ligne, colonne) du curseur dans la zone, base 0.
    pub fn cursor_rc(&self) -> (usize, usize) {
        (self.cursor / self.cols, self.cursor % self.cols)
    }

    /// Dernière ligne (base 0) contenant du texte (0 si vide).
    pub fn last_row(&self) -> usize {
        if self.buf.is_empty() {
            0
        } else {
            (self.buf.len() - 1) / self.cols
        }
    }

    /// Projette le buffer en lignes de largeur `cols` (découpe fixe).
    pub fn lines(&self) -> Vec<String> {
        if self.buf.is_empty() {
            return vec![String::new()];
        }
        self.buf
            .chunks(self.cols)
            .map(|c| c.iter().collect())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_text() {
        let mut e = Editor::new(40, 4);
        for c in "café".chars() {
            e.insert(c);
        }
        assert_eq!(e.text(), "café");
        assert_eq!(e.cursor_rc(), (0, 4));
    }

    #[test]
    fn wrap_by_width() {
        let mut e = Editor::new(5, 3);
        for c in "abcdefg".chars() {
            e.insert(c);
        }
        assert_eq!(e.lines(), vec!["abcde", "fg"]);
        assert_eq!(e.cursor_rc(), (1, 2)); // 7e char -> ligne 1 col 2
        assert_eq!(e.last_row(), 1);
    }

    #[test]
    fn backspace_and_arrows() {
        let mut e = Editor::new(10, 2);
        for c in "abcd".chars() {
            e.insert(c);
        }
        e.left();
        e.left(); // curseur entre b et c
        assert_eq!(e.cursor_rc(), (0, 2));
        e.backspace(); // efface 'b'
        assert_eq!(e.text(), "acd");
        assert_eq!(e.cursor_rc(), (0, 1));
        e.insert('X'); // insertion au milieu
        assert_eq!(e.text(), "aXcd");
    }

    #[test]
    fn capacity_full() {
        let mut e = Editor::new(2, 1); // 2 chars max
        assert!(e.insert('a'));
        assert!(e.insert('b'));
        assert!(!e.insert('c')); // plein
        assert_eq!(e.text(), "ab");
    }
}
