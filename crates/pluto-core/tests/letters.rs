use pluto_core::font::FONT;
use pluto_core::letters::LETTER_SEGS;

struct Dump(Vec<(u8, u8, bool)>);
impl pluto_core::Display for Dump {
    fn clear_all(&mut self) {
        self.0.clear();
    }
    fn set_segment(&mut self, com: u8, seg: u8, on: bool) {
        self.0.push((com, seg, on));
    }
}

#[test]
fn letters_are_faithful() {
    println!("FONT[1]={:?}", FONT[1]);

    let mut d = Dump(Vec::new());
    pluto_core::DigitDisplay::set_char(&mut d, 1, b'H');
    let on: Vec<_> = d
        .0
        .iter()
        .filter(|s| s.2)
        .map(|s| (s.0, s.1))
        .collect();
    println!("pos1 H => {:?}", on);
    // 'H' on the right digit: B/C share (1,11), E/F share (1,12), G = (2,12).
    // A(0,11) and D(2,11) must stay off.
    assert!(on.contains(&(1, 11)) && on.contains(&(1, 12)) && on.contains(&(2, 12)));
    assert!(!on.contains(&(0, 11)) && !on.contains(&(2, 11)));

    let mut d = Dump(Vec::new());
    pluto_core::DigitDisplay::set_char(&mut d, 0, b'T');
    let on: Vec<_> = d
        .0
        .iter()
        .filter(|s| s.2)
        .map(|s| (s.0, s.1))
        .collect();
    println!("pos0 T => {:?}", on);
    // 'T' on the left digit: top (0,13) + the center vertical (1,14).
    assert!(on.contains(&(0, 13)) && on.contains(&(1, 14)));
    assert_eq!(on.len(), 2);

    let mut d = Dump(Vec::new());
    pluto_core::DigitDisplay::set_char(&mut d, 1, b'R');
    let on: Vec<_> = d
        .0
        .iter()
        .filter(|s| s.2)
        .map(|s| (s.0, s.1))
        .collect();
    println!("pos1 R => {:?}", on);
    // 'R' on the right digit lights the ninth (0,12) but not D(2,11).
    assert!(on.contains(&(0, 11)) && on.contains(&(1, 11)) && on.contains(&(1, 12)) && on.contains(&(2, 12)));
    assert!(on.contains(&(0, 12)), "R must light the ninth (0,12)");
    assert!(!on.contains(&(2, 11)), "R must NOT light D(2,11)");
}

#[test]
fn letters_table_is_indexed_by_ascii() {
    // The table is indexed by (ch - 0x20) and covers every printable char.
    assert_eq!(LETTER_SEGS.len(), 95);
    assert_eq!(LETTER_SEGS[b'A' as usize - 0x20].len(), 2);
    assert_eq!(LETTER_SEGS[b' ' as usize - 0x20], [&[][..], &[][..]]);
}
