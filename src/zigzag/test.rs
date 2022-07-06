use {super::*, ::expect_test::expect_file};

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
    assert_eq!(0_u8.zigzag(), 0_i8);
    assert_eq!(0_u16.zigzag(), 0_i16);
    assert_eq!(0_u32.zigzag(), 0_i32);
    assert_eq!(0_u64.zigzag(), 0_i64);
    assert_eq!(0_u128.zigzag(), 0_i128);
    assert_eq!(0_usize.zigzag(), 0_isize);

    assert_eq!(1_u8.zigzag(), 1_i8);
    assert_eq!(2_u8.zigzag(), -1_i8);
    assert_eq!(3_u8.zigzag(), 2_i8);
    assert_eq!(4_u8.zigzag(), -2_i8);
    assert_eq!(5_u8.zigzag(), 3_i8);

    assert_eq!(125_u8.zigzag(), 63_i8);
    assert_eq!(126_u8.zigzag(), -63_i8);
    assert_eq!(127_u8.zigzag(), 64_i8);
    assert_eq!(128_u8.zigzag(), -64i8);
    assert_eq!(129_u8.zigzag(), 65i8);
    assert_eq!(130_u8.zigzag(), -65i8);

    assert_eq!(250_u8.zigzag(), -125_i8);
    assert_eq!(251_u8.zigzag(), 126_i8);
    assert_eq!(252_u8.zigzag(), -126_i8);
    assert_eq!(253_u8.zigzag(), 127_i8);
    assert_eq!(254_u8.zigzag(), -127_i8);

    assert_eq!(255_u8.zigzag(), -128_i8);
    assert_eq!(255_u16.zigzag(), 128_i16);
    assert_eq!(255_u32.zigzag(), 128_i32);
    assert_eq!(255_u64.zigzag(), 128_i64);
    assert_eq!(255_u128.zigzag(), 128_i128);
    assert_eq!(255_usize.zigzag(), 128_isize);

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

    expect_file("zigzag.txt").assert_eq(&actual);
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
    assert_eq!(0_u8.zugzug(), (0_i8, 0_i8));
    assert_eq!(1_u8.zugzug(), (0_i8, 1_i8));
    assert_eq!(2_u8.zugzug(), (1_i8, 1_i8));
    assert_eq!(3_u8.zugzug(), (-1_i8, 0_i8));
    assert_eq!(4_u8.zugzug(), (-1_i8, 1_i8));
    assert_eq!(5_u8.zugzug(), (-1_i8, -1_i8));
    assert_eq!(6_u8.zugzug(), (0_i8, 2_i8));
    assert_eq!(7_u8.zugzug(), (1_i8, 2_i8));
    assert_eq!(8_u8.zugzug(), (-1_i8, 2_i8));
    assert_eq!(9_u8.zugzug(), (2_i8, 2_i8));
    assert_eq!(10_u8.zugzug(), (-2_i8, 0_i8));
    assert_eq!(11_u8.zugzug(), (-2_i8, 1_i8));

    assert_eq!(254_u8.zugzug(), (-11_i8, 1_i8));
    assert_eq!(255_u8.zugzug(), (-11_i8, -1_i8));

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

    expect_file("zugzug.txt").assert_eq(&actual);
}
