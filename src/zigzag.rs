#[cfg(test)]
use ::{expect_test::expect, plotlib};

pub trait ZigZag {
    type Output;

    fn zigzag(self) -> Self::Output;
}

pub trait ZugZug {
    type Output;

    fn zugzug(self) -> Self::Output;
}

macro_rules! impls {
    {$( $signed:ident <-> $unsigned:ident; )+} => {$(
        impl ZigZag for $unsigned {
            type Output = $signed;

            #[inline(always)]
            fn zigzag(self) -> $signed {
                ((self >> 1) as $signed ^ (self as $signed & 1).wrapping_neg()).wrapping_neg()
            }
        }

        impl ZigZag for $signed {
            type Output = $unsigned;

            #[inline(always)]
            fn zigzag(self) -> $unsigned {
                const SIZE_MINUS_ONE: u32 = ::core::mem::size_of::<$unsigned>() as u32 * 8 - 1;
                (((self.wrapping_neg() as $unsigned) << 1) ^ (self.wrapping_neg() >> SIZE_MINUS_ONE) as $unsigned)
            }
        }

        impl ZugZug for $unsigned {
            type Output = ($signed, $signed);

            #[inline(always)]
            fn zugzug(self) -> ($signed, $signed) {
                // XXX: this doesn't do what you think it does

                let s = (self as f64).sqrt() as $unsigned;
                let lo = s.zigzag();
                let hi = ((self - s * s) / 2).zigzag();
                if lo <= hi {
                    (lo, hi)
                } else {
                    (hi, lo)
                }
            }
        }

        impl ZugZug for ($signed, $signed) {
            type Output = $unsigned;

            #[inline(always)]
            fn zugzug(self) -> $unsigned {
                let (lo, hi) = self;
                debug_assert!(lo <= hi);
                todo!()
            }
        }
    )+};
}

impls! {
       i8 <->    u8;
      i16 <->   u16;
      i32 <->   u32;
      i64 <->   u64;
     i128 <->  u128;
    isize <-> usize;
}

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

#[cfg(test)]
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

#[cfg(test)]
#[test]
fn zugzug_round_trip() {
    for uint in u8::MIN..=u8::MAX {
        assert_eq!(uint, uint.zugzug().zugzug());
    }

    for uint in u16::MIN..=u16::MAX {
        assert_eq!(uint, uint.zugzug().zugzug());
    }
}

#[cfg(test)]
#[test]
fn test_zugzug_known_values() {
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

    assert_eq!(254_u8.zugzug(), (0_i8, 0_i8));
    assert_eq!(255_u8.zugzug(), (0_i8, 0_i8));

    assert_eq!(u8::MAX.zugzug(), (0_i8, 0_i8));
    assert_eq!(u16::MAX.zugzug(), (0_i16, 0_i16));
    assert_eq!(u32::MAX.zugzug(), (0_i32, 0_i32));
    assert_eq!(u64::MAX.zugzug(), (0_i64, 0_i64));
    assert_eq!(u128::MAX.zugzug(), (0_i128, 0_i128));
    assert_eq!(usize::MAX.zugzug(), (0_isize, 0_isize));
}

#[cfg(test)]
#[test]
fn test_zugzug_snapshot() {
    let mut actual = String::new();
    let mut existing = std::collections::HashMap::new();
    for i in u8::MIN..=u8::MAX {
        let (x, y) = i.zugzug();
        let previous = existing.insert((x, y), i);
        actual += &format!("{i:>3}: ({x:>3}, {y:>3})");
        if let Some(index) = previous {
            actual += &format!("  ERROR: {i:>3} IS A DUPLICATE OF {index:>3}!");
        }
        actual += "\n";
    }

    expect![[r#"
          0: (  0,   0)
          1: (  0,   1)
          2: (  0,   1)  ERROR:   2 IS A DUPLICATE OF   1!
          3: (  1,   1)
          4: ( -1,   0)
          5: ( -1,   0)  ERROR:   5 IS A DUPLICATE OF   4!
          6: ( -1,   1)
          7: ( -1,   1)  ERROR:   7 IS A DUPLICATE OF   6!
          8: ( -1,  -1)
          9: (  0,   2)
         10: (  0,   2)  ERROR:  10 IS A DUPLICATE OF   9!
         11: (  1,   2)
         12: (  1,   2)  ERROR:  12 IS A DUPLICATE OF  11!
         13: ( -1,   2)
         14: ( -1,   2)  ERROR:  14 IS A DUPLICATE OF  13!
         15: (  2,   2)
         16: ( -2,   0)
         17: ( -2,   0)  ERROR:  17 IS A DUPLICATE OF  16!
         18: ( -2,   1)
         19: ( -2,   1)  ERROR:  19 IS A DUPLICATE OF  18!
         20: ( -2,  -1)
         21: ( -2,  -1)  ERROR:  21 IS A DUPLICATE OF  20!
         22: ( -2,   2)
         23: ( -2,   2)  ERROR:  23 IS A DUPLICATE OF  22!
         24: ( -2,  -2)
         25: (  0,   3)
         26: (  0,   3)  ERROR:  26 IS A DUPLICATE OF  25!
         27: (  1,   3)
         28: (  1,   3)  ERROR:  28 IS A DUPLICATE OF  27!
         29: ( -1,   3)
         30: ( -1,   3)  ERROR:  30 IS A DUPLICATE OF  29!
         31: (  2,   3)
         32: (  2,   3)  ERROR:  32 IS A DUPLICATE OF  31!
         33: ( -2,   3)
         34: ( -2,   3)  ERROR:  34 IS A DUPLICATE OF  33!
         35: (  3,   3)
         36: ( -3,   0)
         37: ( -3,   0)  ERROR:  37 IS A DUPLICATE OF  36!
         38: ( -3,   1)
         39: ( -3,   1)  ERROR:  39 IS A DUPLICATE OF  38!
         40: ( -3,  -1)
         41: ( -3,  -1)  ERROR:  41 IS A DUPLICATE OF  40!
         42: ( -3,   2)
         43: ( -3,   2)  ERROR:  43 IS A DUPLICATE OF  42!
         44: ( -3,  -2)
         45: ( -3,  -2)  ERROR:  45 IS A DUPLICATE OF  44!
         46: ( -3,   3)
         47: ( -3,   3)  ERROR:  47 IS A DUPLICATE OF  46!
         48: ( -3,  -3)
         49: (  0,   4)
         50: (  0,   4)  ERROR:  50 IS A DUPLICATE OF  49!
         51: (  1,   4)
         52: (  1,   4)  ERROR:  52 IS A DUPLICATE OF  51!
         53: ( -1,   4)
         54: ( -1,   4)  ERROR:  54 IS A DUPLICATE OF  53!
         55: (  2,   4)
         56: (  2,   4)  ERROR:  56 IS A DUPLICATE OF  55!
         57: ( -2,   4)
         58: ( -2,   4)  ERROR:  58 IS A DUPLICATE OF  57!
         59: (  3,   4)
         60: (  3,   4)  ERROR:  60 IS A DUPLICATE OF  59!
         61: ( -3,   4)
         62: ( -3,   4)  ERROR:  62 IS A DUPLICATE OF  61!
         63: (  4,   4)
         64: ( -4,   0)
         65: ( -4,   0)  ERROR:  65 IS A DUPLICATE OF  64!
         66: ( -4,   1)
         67: ( -4,   1)  ERROR:  67 IS A DUPLICATE OF  66!
         68: ( -4,  -1)
         69: ( -4,  -1)  ERROR:  69 IS A DUPLICATE OF  68!
         70: ( -4,   2)
         71: ( -4,   2)  ERROR:  71 IS A DUPLICATE OF  70!
         72: ( -4,  -2)
         73: ( -4,  -2)  ERROR:  73 IS A DUPLICATE OF  72!
         74: ( -4,   3)
         75: ( -4,   3)  ERROR:  75 IS A DUPLICATE OF  74!
         76: ( -4,  -3)
         77: ( -4,  -3)  ERROR:  77 IS A DUPLICATE OF  76!
         78: ( -4,   4)
         79: ( -4,   4)  ERROR:  79 IS A DUPLICATE OF  78!
         80: ( -4,  -4)
         81: (  0,   5)
         82: (  0,   5)  ERROR:  82 IS A DUPLICATE OF  81!
         83: (  1,   5)
         84: (  1,   5)  ERROR:  84 IS A DUPLICATE OF  83!
         85: ( -1,   5)
         86: ( -1,   5)  ERROR:  86 IS A DUPLICATE OF  85!
         87: (  2,   5)
         88: (  2,   5)  ERROR:  88 IS A DUPLICATE OF  87!
         89: ( -2,   5)
         90: ( -2,   5)  ERROR:  90 IS A DUPLICATE OF  89!
         91: (  3,   5)
         92: (  3,   5)  ERROR:  92 IS A DUPLICATE OF  91!
         93: ( -3,   5)
         94: ( -3,   5)  ERROR:  94 IS A DUPLICATE OF  93!
         95: (  4,   5)
         96: (  4,   5)  ERROR:  96 IS A DUPLICATE OF  95!
         97: ( -4,   5)
         98: ( -4,   5)  ERROR:  98 IS A DUPLICATE OF  97!
         99: (  5,   5)
        100: ( -5,   0)
        101: ( -5,   0)  ERROR: 101 IS A DUPLICATE OF 100!
        102: ( -5,   1)
        103: ( -5,   1)  ERROR: 103 IS A DUPLICATE OF 102!
        104: ( -5,  -1)
        105: ( -5,  -1)  ERROR: 105 IS A DUPLICATE OF 104!
        106: ( -5,   2)
        107: ( -5,   2)  ERROR: 107 IS A DUPLICATE OF 106!
        108: ( -5,  -2)
        109: ( -5,  -2)  ERROR: 109 IS A DUPLICATE OF 108!
        110: ( -5,   3)
        111: ( -5,   3)  ERROR: 111 IS A DUPLICATE OF 110!
        112: ( -5,  -3)
        113: ( -5,  -3)  ERROR: 113 IS A DUPLICATE OF 112!
        114: ( -5,   4)
        115: ( -5,   4)  ERROR: 115 IS A DUPLICATE OF 114!
        116: ( -5,  -4)
        117: ( -5,  -4)  ERROR: 117 IS A DUPLICATE OF 116!
        118: ( -5,   5)
        119: ( -5,   5)  ERROR: 119 IS A DUPLICATE OF 118!
        120: ( -5,  -5)
        121: (  0,   6)
        122: (  0,   6)  ERROR: 122 IS A DUPLICATE OF 121!
        123: (  1,   6)
        124: (  1,   6)  ERROR: 124 IS A DUPLICATE OF 123!
        125: ( -1,   6)
        126: ( -1,   6)  ERROR: 126 IS A DUPLICATE OF 125!
        127: (  2,   6)
        128: (  2,   6)  ERROR: 128 IS A DUPLICATE OF 127!
        129: ( -2,   6)
        130: ( -2,   6)  ERROR: 130 IS A DUPLICATE OF 129!
        131: (  3,   6)
        132: (  3,   6)  ERROR: 132 IS A DUPLICATE OF 131!
        133: ( -3,   6)
        134: ( -3,   6)  ERROR: 134 IS A DUPLICATE OF 133!
        135: (  4,   6)
        136: (  4,   6)  ERROR: 136 IS A DUPLICATE OF 135!
        137: ( -4,   6)
        138: ( -4,   6)  ERROR: 138 IS A DUPLICATE OF 137!
        139: (  5,   6)
        140: (  5,   6)  ERROR: 140 IS A DUPLICATE OF 139!
        141: ( -5,   6)
        142: ( -5,   6)  ERROR: 142 IS A DUPLICATE OF 141!
        143: (  6,   6)
        144: ( -6,   0)
        145: ( -6,   0)  ERROR: 145 IS A DUPLICATE OF 144!
        146: ( -6,   1)
        147: ( -6,   1)  ERROR: 147 IS A DUPLICATE OF 146!
        148: ( -6,  -1)
        149: ( -6,  -1)  ERROR: 149 IS A DUPLICATE OF 148!
        150: ( -6,   2)
        151: ( -6,   2)  ERROR: 151 IS A DUPLICATE OF 150!
        152: ( -6,  -2)
        153: ( -6,  -2)  ERROR: 153 IS A DUPLICATE OF 152!
        154: ( -6,   3)
        155: ( -6,   3)  ERROR: 155 IS A DUPLICATE OF 154!
        156: ( -6,  -3)
        157: ( -6,  -3)  ERROR: 157 IS A DUPLICATE OF 156!
        158: ( -6,   4)
        159: ( -6,   4)  ERROR: 159 IS A DUPLICATE OF 158!
        160: ( -6,  -4)
        161: ( -6,  -4)  ERROR: 161 IS A DUPLICATE OF 160!
        162: ( -6,   5)
        163: ( -6,   5)  ERROR: 163 IS A DUPLICATE OF 162!
        164: ( -6,  -5)
        165: ( -6,  -5)  ERROR: 165 IS A DUPLICATE OF 164!
        166: ( -6,   6)
        167: ( -6,   6)  ERROR: 167 IS A DUPLICATE OF 166!
        168: ( -6,  -6)
        169: (  0,   7)
        170: (  0,   7)  ERROR: 170 IS A DUPLICATE OF 169!
        171: (  1,   7)
        172: (  1,   7)  ERROR: 172 IS A DUPLICATE OF 171!
        173: ( -1,   7)
        174: ( -1,   7)  ERROR: 174 IS A DUPLICATE OF 173!
        175: (  2,   7)
        176: (  2,   7)  ERROR: 176 IS A DUPLICATE OF 175!
        177: ( -2,   7)
        178: ( -2,   7)  ERROR: 178 IS A DUPLICATE OF 177!
        179: (  3,   7)
        180: (  3,   7)  ERROR: 180 IS A DUPLICATE OF 179!
        181: ( -3,   7)
        182: ( -3,   7)  ERROR: 182 IS A DUPLICATE OF 181!
        183: (  4,   7)
        184: (  4,   7)  ERROR: 184 IS A DUPLICATE OF 183!
        185: ( -4,   7)
        186: ( -4,   7)  ERROR: 186 IS A DUPLICATE OF 185!
        187: (  5,   7)
        188: (  5,   7)  ERROR: 188 IS A DUPLICATE OF 187!
        189: ( -5,   7)
        190: ( -5,   7)  ERROR: 190 IS A DUPLICATE OF 189!
        191: (  6,   7)
        192: (  6,   7)  ERROR: 192 IS A DUPLICATE OF 191!
        193: ( -6,   7)
        194: ( -6,   7)  ERROR: 194 IS A DUPLICATE OF 193!
        195: (  7,   7)
        196: ( -7,   0)
        197: ( -7,   0)  ERROR: 197 IS A DUPLICATE OF 196!
        198: ( -7,   1)
        199: ( -7,   1)  ERROR: 199 IS A DUPLICATE OF 198!
        200: ( -7,  -1)
        201: ( -7,  -1)  ERROR: 201 IS A DUPLICATE OF 200!
        202: ( -7,   2)
        203: ( -7,   2)  ERROR: 203 IS A DUPLICATE OF 202!
        204: ( -7,  -2)
        205: ( -7,  -2)  ERROR: 205 IS A DUPLICATE OF 204!
        206: ( -7,   3)
        207: ( -7,   3)  ERROR: 207 IS A DUPLICATE OF 206!
        208: ( -7,  -3)
        209: ( -7,  -3)  ERROR: 209 IS A DUPLICATE OF 208!
        210: ( -7,   4)
        211: ( -7,   4)  ERROR: 211 IS A DUPLICATE OF 210!
        212: ( -7,  -4)
        213: ( -7,  -4)  ERROR: 213 IS A DUPLICATE OF 212!
        214: ( -7,   5)
        215: ( -7,   5)  ERROR: 215 IS A DUPLICATE OF 214!
        216: ( -7,  -5)
        217: ( -7,  -5)  ERROR: 217 IS A DUPLICATE OF 216!
        218: ( -7,   6)
        219: ( -7,   6)  ERROR: 219 IS A DUPLICATE OF 218!
        220: ( -7,  -6)
        221: ( -7,  -6)  ERROR: 221 IS A DUPLICATE OF 220!
        222: ( -7,   7)
        223: ( -7,   7)  ERROR: 223 IS A DUPLICATE OF 222!
        224: ( -7,  -7)
        225: (  0,   8)
        226: (  0,   8)  ERROR: 226 IS A DUPLICATE OF 225!
        227: (  1,   8)
        228: (  1,   8)  ERROR: 228 IS A DUPLICATE OF 227!
        229: ( -1,   8)
        230: ( -1,   8)  ERROR: 230 IS A DUPLICATE OF 229!
        231: (  2,   8)
        232: (  2,   8)  ERROR: 232 IS A DUPLICATE OF 231!
        233: ( -2,   8)
        234: ( -2,   8)  ERROR: 234 IS A DUPLICATE OF 233!
        235: (  3,   8)
        236: (  3,   8)  ERROR: 236 IS A DUPLICATE OF 235!
        237: ( -3,   8)
        238: ( -3,   8)  ERROR: 238 IS A DUPLICATE OF 237!
        239: (  4,   8)
        240: (  4,   8)  ERROR: 240 IS A DUPLICATE OF 239!
        241: ( -4,   8)
        242: ( -4,   8)  ERROR: 242 IS A DUPLICATE OF 241!
        243: (  5,   8)
        244: (  5,   8)  ERROR: 244 IS A DUPLICATE OF 243!
        245: ( -5,   8)
        246: ( -5,   8)  ERROR: 246 IS A DUPLICATE OF 245!
        247: (  6,   8)
        248: (  6,   8)  ERROR: 248 IS A DUPLICATE OF 247!
        249: ( -6,   8)
        250: ( -6,   8)  ERROR: 250 IS A DUPLICATE OF 249!
        251: (  7,   8)
        252: (  7,   8)  ERROR: 252 IS A DUPLICATE OF 251!
        253: ( -7,   8)
        254: ( -7,   8)  ERROR: 254 IS A DUPLICATE OF 253!
        255: (  8,   8)
    "#]]
    .assert_eq(&actual);
}

#[cfg(test)]
#[test]
fn test_zugzug_plot() {
    use plotlib::{
        page::Page,
        repr::Plot,
        style::{PointMarker, PointStyle},
        view::ContinuousView,
    };

    let mut actual = String::new();

    let mut view = ContinuousView::new();

    let markers = [PointMarker::Circle, PointMarker::Square, PointMarker::Cross];

    for (range, marker) in [
        (0..16, markers[0]),
        (16..32, markers[1]),
        (32..48, markers[2]),
    ] {
        let data = range
            .map(|x: u64| {
                let (l, r) = x.zugzug();
                (l as f64, r as f64)
            })
            .collect();

        let plot = Plot::new(data).point_style(PointStyle::new().marker(marker));

        view = view.add(plot);
    }

    actual += Page::single(&view)
        .dimensions(60, 20)
        .to_text()
        .unwrap()
        .trim_end_matches(' ');

    expect![[r#"
            3-|        ×         ■         ■         ■         ×         ×
              |
              |
              |
            2-|        ■         ●         ●         ●         ●
              |
              |
              |
            1-|        ■         ●         ●         ●
              |
              |
              |
            0-|        ■         ●         ●
              |
              |
              |
           -1-|        ■         ●
              |
              |
              |
           -2+------------------------------------------------------------
                       |                   |                   |
                      -2                   0                   2
    "#]]
    .assert_eq(&actual);
}
