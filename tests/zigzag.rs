use ::save::{
    testing::{assert_at, assert_debug_eq},
    zigzag::{ZigZag, ZugZug},
};

#[test]
fn zigzag_round_trip() {
    for uint in u8::MIN..=u8::MAX {
        assert_eq!(uint, uint.zigzag().zigzag());
    }

    for int in i8::MIN..=i8::MAX {
        assert_eq!(int, int.zigzag().zigzag());
    }

    for uint in u16::MIN..=u16::MAX {
        assert_eq!(uint, uint.zigzag().zigzag());
    }

    for int in i16::MIN..=i16::MAX {
        assert_eq!(int, int.zigzag().zigzag());
    }
}

#[test]
fn zigzag_known_values() {
    assert_debug_eq("0", 0_u8.zigzag());
    assert_debug_eq("0", 0_u16.zigzag());
    assert_debug_eq("0", 0_u32.zigzag());
    assert_debug_eq("0", 0_u64.zigzag());
    assert_debug_eq("0", 0_u128.zigzag());
    assert_debug_eq("0", 0_usize.zigzag());

    assert_debug_eq("1", 1_u8.zigzag());
    assert_debug_eq("-1", 2_u8.zigzag());
    assert_debug_eq("2", 3_u8.zigzag());
    assert_debug_eq("-2", 4_u8.zigzag());
    assert_debug_eq("3", 5_u8.zigzag());

    assert_debug_eq("63", 125_u8.zigzag());
    assert_debug_eq("-63", 126_u8.zigzag());
    assert_debug_eq("64", 127_u8.zigzag());
    assert_debug_eq("-64", 128_u8.zigzag());
    assert_debug_eq("65", 129_u8.zigzag());
    assert_debug_eq("-65", 130_u8.zigzag());

    assert_debug_eq("-125", 250_u8.zigzag());
    assert_debug_eq("126", 251_u8.zigzag());
    assert_debug_eq("-126", 252_u8.zigzag());
    assert_debug_eq("127", 253_u8.zigzag());
    assert_debug_eq("-127", 254_u8.zigzag());

    assert_debug_eq("-128", 255_u8.zigzag());
    assert_debug_eq("128", 255_u16.zigzag());
    assert_debug_eq("128", 255_u32.zigzag());
    assert_debug_eq("128", 255_u64.zigzag());
    assert_debug_eq("128", 255_u128.zigzag());
    assert_debug_eq("128", 255_usize.zigzag());

    assert_eq!(u8::MAX.zigzag(), i8::MIN);
    assert_eq!(u16::MAX.zigzag(), i16::MIN);
    assert_eq!(u32::MAX.zigzag(), i32::MIN);
    assert_eq!(u64::MAX.zigzag(), i64::MIN);
    assert_eq!(u128::MAX.zigzag(), i128::MIN);
    assert_eq!(usize::MAX.zigzag(), isize::MIN);

    assert_eq!((u8::MAX - 2).zigzag(), i8::MAX);
    assert_eq!((u16::MAX - 2).zigzag(), i16::MAX);
    assert_eq!((u32::MAX - 2).zigzag(), i32::MAX);
    assert_eq!((u64::MAX - 2).zigzag(), i64::MAX);
    assert_eq!((u128::MAX - 2).zigzag(), i128::MAX);
    assert_eq!((usize::MAX - 2).zigzag(), isize::MAX);
}

#[test]
fn zigzag_snapshot() {
    let mut actual = String::new();

    for i in u8::MIN..=u8::MAX {
        let x = i.zigzag();
        let xp = (i as u16).zigzag();
        actual += &format!("{i:>4}_u8.zigzag() == {x:>4}_i8");
        if xp != (x as _) {
            actual += &format!(", but {i:>4}_u16.zigzag() == {xp:>4}_i16");
        }
        actual += "\n";
    }

    actual += "\n";

    for i in i8::MIN..=i8::MAX {
        let x = i.zigzag();
        let xp = (i as i16).zigzag();
        actual += &format!("{i:>4}_i8.zigzag() == {x:>4}_u8");
        if xp != (x as _) {
            actual += &format!(", but {i:>4}_i16.zigzag() == {xp:>4}_u16");
        }
        actual += "\n";
    }

    assert_at("zigzag.txt", &actual);
}

#[test]
fn zugzug_round_trip() {
    for uint in u8::MIN..=u8::MAX {
        assert_eq!(uint, uint.zugzug().zugzug());
    }

    for int in i8::MIN..=i8::MAX {
        assert_eq!(int, int.zigzag().zugzug().zugzug().zigzag());
    }

    for uint in u16::MIN..=u16::MAX {
        assert_eq!(uint, uint.zugzug().zugzug());
    }

    for int in i16::MIN..=i16::MAX {
        assert_eq!(int, int.zigzag().zugzug().zugzug().zigzag());
    }
}

#[test]
fn zugzug_known_values() {
    assert_debug_eq("(0, 0)", 0_u8.zugzug());
    assert_debug_eq("(0, 1)", 1_u8.zugzug());
    assert_debug_eq("(1, 1)", 2_u8.zugzug());
    assert_debug_eq("(-1, 0)", 3_u8.zugzug());
    assert_debug_eq("(-1, 1)", 4_u8.zugzug());
    assert_debug_eq("(-1, -1)", 5_u8.zugzug());
    assert_debug_eq("(0, 2)", 6_u8.zugzug());
    assert_debug_eq("(1, 2)", 7_u8.zugzug());
    assert_debug_eq("(-1, 2)", 8_u8.zugzug());
    assert_debug_eq("(2, 2)", 9_u8.zugzug());
    assert_debug_eq("(-2, 0)", 10_u8.zugzug());
    assert_debug_eq("(-2, 1)", 11_u8.zugzug());

    assert_debug_eq("(-11, 1)", 254_u8.zugzug());
    assert_debug_eq("(-11, -1)", 255_u8.zugzug());

    assert_eq!(u8::MAX.zugzug(), (-11_i8, -1_i8));
    assert_eq!(u16::MAX.zugzug(), (-97_i16, 181_i16));
    assert_eq!(u32::MAX.zugzug(), (-18537_i32, 46341_i32));
    assert_eq!(u64::MAX.zugzug(), (1373026058_i64, 3037000500_i64));
    assert_eq!(usize::MAX.zugzug(), (1373026058_isize, 3037000500_isize));
}

#[test]
fn zugzug_snapshot() {
    let mut actual = String::new();

    for i in 0..1024u16 {
        let (x, y) = i.zugzug();
        actual += &format!("{i:>4}.zugzug() == ({x:>3}, {y:>3})\n");
    }

    actual += "\n";

    for x in -16..16i16 {
        for y in x..32i16 {
            let i = (x, y).zugzug();
            actual += &format!("({x:>3}, {y:>3}).zugzug() == {i:>4}\n");
        }
    }

    assert_at("zugzug.txt", &actual);
}
