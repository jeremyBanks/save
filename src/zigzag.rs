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
                let hi = (self - s * s).zigzag();
                (lo, hi)
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
fn test_zigzag_snapshot() {
    let mut actual = String::new();
    let mut existing = std::collections::HashMap::new();
    for i in u8::MIN..=u8::MAX {
        let x = i.zigzag();
        let previous = existing.insert(x, i);
        actual += &format!("{i:>3}: {x:>4}");
        if let Some(index) = previous {
            actual += &format!("  ERROR: {i:>3} IS A DUPLICATE OF {index:>3}!");
        }
        actual += "\n";
    }

    expect![[r#"
          0:    0
          1:    1
          2:   -1
          3:    2
          4:   -2
          5:    3
          6:   -3
          7:    4
          8:   -4
          9:    5
         10:   -5
         11:    6
         12:   -6
         13:    7
         14:   -7
         15:    8
         16:   -8
         17:    9
         18:   -9
         19:   10
         20:  -10
         21:   11
         22:  -11
         23:   12
         24:  -12
         25:   13
         26:  -13
         27:   14
         28:  -14
         29:   15
         30:  -15
         31:   16
         32:  -16
         33:   17
         34:  -17
         35:   18
         36:  -18
         37:   19
         38:  -19
         39:   20
         40:  -20
         41:   21
         42:  -21
         43:   22
         44:  -22
         45:   23
         46:  -23
         47:   24
         48:  -24
         49:   25
         50:  -25
         51:   26
         52:  -26
         53:   27
         54:  -27
         55:   28
         56:  -28
         57:   29
         58:  -29
         59:   30
         60:  -30
         61:   31
         62:  -31
         63:   32
         64:  -32
         65:   33
         66:  -33
         67:   34
         68:  -34
         69:   35
         70:  -35
         71:   36
         72:  -36
         73:   37
         74:  -37
         75:   38
         76:  -38
         77:   39
         78:  -39
         79:   40
         80:  -40
         81:   41
         82:  -41
         83:   42
         84:  -42
         85:   43
         86:  -43
         87:   44
         88:  -44
         89:   45
         90:  -45
         91:   46
         92:  -46
         93:   47
         94:  -47
         95:   48
         96:  -48
         97:   49
         98:  -49
         99:   50
        100:  -50
        101:   51
        102:  -51
        103:   52
        104:  -52
        105:   53
        106:  -53
        107:   54
        108:  -54
        109:   55
        110:  -55
        111:   56
        112:  -56
        113:   57
        114:  -57
        115:   58
        116:  -58
        117:   59
        118:  -59
        119:   60
        120:  -60
        121:   61
        122:  -61
        123:   62
        124:  -62
        125:   63
        126:  -63
        127:   64
        128:  -64
        129:   65
        130:  -65
        131:   66
        132:  -66
        133:   67
        134:  -67
        135:   68
        136:  -68
        137:   69
        138:  -69
        139:   70
        140:  -70
        141:   71
        142:  -71
        143:   72
        144:  -72
        145:   73
        146:  -73
        147:   74
        148:  -74
        149:   75
        150:  -75
        151:   76
        152:  -76
        153:   77
        154:  -77
        155:   78
        156:  -78
        157:   79
        158:  -79
        159:   80
        160:  -80
        161:   81
        162:  -81
        163:   82
        164:  -82
        165:   83
        166:  -83
        167:   84
        168:  -84
        169:   85
        170:  -85
        171:   86
        172:  -86
        173:   87
        174:  -87
        175:   88
        176:  -88
        177:   89
        178:  -89
        179:   90
        180:  -90
        181:   91
        182:  -91
        183:   92
        184:  -92
        185:   93
        186:  -93
        187:   94
        188:  -94
        189:   95
        190:  -95
        191:   96
        192:  -96
        193:   97
        194:  -97
        195:   98
        196:  -98
        197:   99
        198:  -99
        199:  100
        200: -100
        201:  101
        202: -101
        203:  102
        204: -102
        205:  103
        206: -103
        207:  104
        208: -104
        209:  105
        210: -105
        211:  106
        212: -106
        213:  107
        214: -107
        215:  108
        216: -108
        217:  109
        218: -109
        219:  110
        220: -110
        221:  111
        222: -111
        223:  112
        224: -112
        225:  113
        226: -113
        227:  114
        228: -114
        229:  115
        230: -115
        231:  116
        232: -116
        233:  117
        234: -117
        235:  118
        236: -118
        237:  119
        238: -119
        239:  120
        240: -120
        241:  121
        242: -121
        243:  122
        244: -122
        245:  123
        246: -123
        247:  124
        248: -124
        249:  125
        250: -125
        251:  126
        252: -126
        253:  127
        254: -127
        255: -128
    "#]]
    .assert_eq(&actual);
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
          1: (  1,   0)
          2: (  1,   1)
          3: (  1,  -1)
          4: ( -1,   0)
          5: ( -1,   1)
          6: ( -1,  -1)
          7: ( -1,   2)
          8: ( -1,  -2)
          9: (  2,   0)
         10: (  2,   1)
         11: (  2,  -1)
         12: (  2,   2)
         13: (  2,  -2)
         14: (  2,   3)
         15: (  2,  -3)
         16: ( -2,   0)
         17: ( -2,   1)
         18: ( -2,  -1)
         19: ( -2,   2)
         20: ( -2,  -2)
         21: ( -2,   3)
         22: ( -2,  -3)
         23: ( -2,   4)
         24: ( -2,  -4)
         25: (  3,   0)
         26: (  3,   1)
         27: (  3,  -1)
         28: (  3,   2)
         29: (  3,  -2)
         30: (  3,   3)
         31: (  3,  -3)
         32: (  3,   4)
         33: (  3,  -4)
         34: (  3,   5)
         35: (  3,  -5)
         36: ( -3,   0)
         37: ( -3,   1)
         38: ( -3,  -1)
         39: ( -3,   2)
         40: ( -3,  -2)
         41: ( -3,   3)
         42: ( -3,  -3)
         43: ( -3,   4)
         44: ( -3,  -4)
         45: ( -3,   5)
         46: ( -3,  -5)
         47: ( -3,   6)
         48: ( -3,  -6)
         49: (  4,   0)
         50: (  4,   1)
         51: (  4,  -1)
         52: (  4,   2)
         53: (  4,  -2)
         54: (  4,   3)
         55: (  4,  -3)
         56: (  4,   4)
         57: (  4,  -4)
         58: (  4,   5)
         59: (  4,  -5)
         60: (  4,   6)
         61: (  4,  -6)
         62: (  4,   7)
         63: (  4,  -7)
         64: ( -4,   0)
         65: ( -4,   1)
         66: ( -4,  -1)
         67: ( -4,   2)
         68: ( -4,  -2)
         69: ( -4,   3)
         70: ( -4,  -3)
         71: ( -4,   4)
         72: ( -4,  -4)
         73: ( -4,   5)
         74: ( -4,  -5)
         75: ( -4,   6)
         76: ( -4,  -6)
         77: ( -4,   7)
         78: ( -4,  -7)
         79: ( -4,   8)
         80: ( -4,  -8)
         81: (  5,   0)
         82: (  5,   1)
         83: (  5,  -1)
         84: (  5,   2)
         85: (  5,  -2)
         86: (  5,   3)
         87: (  5,  -3)
         88: (  5,   4)
         89: (  5,  -4)
         90: (  5,   5)
         91: (  5,  -5)
         92: (  5,   6)
         93: (  5,  -6)
         94: (  5,   7)
         95: (  5,  -7)
         96: (  5,   8)
         97: (  5,  -8)
         98: (  5,   9)
         99: (  5,  -9)
        100: ( -5,   0)
        101: ( -5,   1)
        102: ( -5,  -1)
        103: ( -5,   2)
        104: ( -5,  -2)
        105: ( -5,   3)
        106: ( -5,  -3)
        107: ( -5,   4)
        108: ( -5,  -4)
        109: ( -5,   5)
        110: ( -5,  -5)
        111: ( -5,   6)
        112: ( -5,  -6)
        113: ( -5,   7)
        114: ( -5,  -7)
        115: ( -5,   8)
        116: ( -5,  -8)
        117: ( -5,   9)
        118: ( -5,  -9)
        119: ( -5,  10)
        120: ( -5, -10)
        121: (  6,   0)
        122: (  6,   1)
        123: (  6,  -1)
        124: (  6,   2)
        125: (  6,  -2)
        126: (  6,   3)
        127: (  6,  -3)
        128: (  6,   4)
        129: (  6,  -4)
        130: (  6,   5)
        131: (  6,  -5)
        132: (  6,   6)
        133: (  6,  -6)
        134: (  6,   7)
        135: (  6,  -7)
        136: (  6,   8)
        137: (  6,  -8)
        138: (  6,   9)
        139: (  6,  -9)
        140: (  6,  10)
        141: (  6, -10)
        142: (  6,  11)
        143: (  6, -11)
        144: ( -6,   0)
        145: ( -6,   1)
        146: ( -6,  -1)
        147: ( -6,   2)
        148: ( -6,  -2)
        149: ( -6,   3)
        150: ( -6,  -3)
        151: ( -6,   4)
        152: ( -6,  -4)
        153: ( -6,   5)
        154: ( -6,  -5)
        155: ( -6,   6)
        156: ( -6,  -6)
        157: ( -6,   7)
        158: ( -6,  -7)
        159: ( -6,   8)
        160: ( -6,  -8)
        161: ( -6,   9)
        162: ( -6,  -9)
        163: ( -6,  10)
        164: ( -6, -10)
        165: ( -6,  11)
        166: ( -6, -11)
        167: ( -6,  12)
        168: ( -6, -12)
        169: (  7,   0)
        170: (  7,   1)
        171: (  7,  -1)
        172: (  7,   2)
        173: (  7,  -2)
        174: (  7,   3)
        175: (  7,  -3)
        176: (  7,   4)
        177: (  7,  -4)
        178: (  7,   5)
        179: (  7,  -5)
        180: (  7,   6)
        181: (  7,  -6)
        182: (  7,   7)
        183: (  7,  -7)
        184: (  7,   8)
        185: (  7,  -8)
        186: (  7,   9)
        187: (  7,  -9)
        188: (  7,  10)
        189: (  7, -10)
        190: (  7,  11)
        191: (  7, -11)
        192: (  7,  12)
        193: (  7, -12)
        194: (  7,  13)
        195: (  7, -13)
        196: ( -7,   0)
        197: ( -7,   1)
        198: ( -7,  -1)
        199: ( -7,   2)
        200: ( -7,  -2)
        201: ( -7,   3)
        202: ( -7,  -3)
        203: ( -7,   4)
        204: ( -7,  -4)
        205: ( -7,   5)
        206: ( -7,  -5)
        207: ( -7,   6)
        208: ( -7,  -6)
        209: ( -7,   7)
        210: ( -7,  -7)
        211: ( -7,   8)
        212: ( -7,  -8)
        213: ( -7,   9)
        214: ( -7,  -9)
        215: ( -7,  10)
        216: ( -7, -10)
        217: ( -7,  11)
        218: ( -7, -11)
        219: ( -7,  12)
        220: ( -7, -12)
        221: ( -7,  13)
        222: ( -7, -13)
        223: ( -7,  14)
        224: ( -7, -14)
        225: (  8,   0)
        226: (  8,   1)
        227: (  8,  -1)
        228: (  8,   2)
        229: (  8,  -2)
        230: (  8,   3)
        231: (  8,  -3)
        232: (  8,   4)
        233: (  8,  -4)
        234: (  8,   5)
        235: (  8,  -5)
        236: (  8,   6)
        237: (  8,  -6)
        238: (  8,   7)
        239: (  8,  -7)
        240: (  8,   8)
        241: (  8,  -8)
        242: (  8,   9)
        243: (  8,  -9)
        244: (  8,  10)
        245: (  8, -10)
        246: (  8,  11)
        247: (  8, -11)
        248: (  8,  12)
        249: (  8, -12)
        250: (  8,  13)
        251: (  8, -13)
        252: (  8,  14)
        253: (  8, -14)
        254: (  8,  15)
        255: (  8, -15)
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
    let range = 0..256;
    let data = range
        .clone()
        .enumerate()
        .map(|(i, x): (usize, u64)| {
            let (_l, r) = x.zugzug();
            (r as f64, -((i as u64 + range.start) as f64))
        })
        .collect();

    let plot = Plot::new(data).point_style(PointStyle::new().marker(PointMarker::Circle));

    view = view.add(plot);

    let data = range
        .clone()
        .enumerate()
        .map(|(i, x): (usize, u64)| {
            let (l, _r) = x.zugzug();
            (l as f64, -((i as u64 + range.start) as f64))
        })
        .collect();

    let plot = Plot::new(data).point_style(PointStyle::new().marker(PointMarker::Circle));

    view = view.add(plot);

    actual += Page::single(&view)
        .dimensions(60, 256)
        .to_text()
        .unwrap()
        .trim_end_matches(' ');

    expect![[r#"
              0-|                            ●                              
                |                            ● ●                            
                |                              ●                            
                |                          ●   ●                            
                |                          ● ●                              
                |                          ●   ●                            
                |                          ●                                
                |                          ●     ●                          
                |                        ● ●                                
                |                            ●   ●                          
                |                              ● ●                          
                |                          ●     ●                          
                |                                ●                          
                |                        ●       ●                          
                |                                ● ●                        
                |                      ●         ●                          
                |                        ●   ●                              
                |                        ●     ●                            
                |                        ● ●                                
                |                        ●       ●                          
                |                        ●                                  
                |                        ●         ●                        
                |                      ● ●                                  
                |                        ●           ●                      
                |                    ●   ●                                  
                |                            ●     ●                        
                |                              ●   ●                        
                |                          ●       ●                        
                |                                ● ●                        
                |                        ●         ●                        
                |                                  ●                        
                |                      ●           ●                        
                |                                  ● ●                      
                |                    ●             ●                        
                |                                  ●   ●                    
                |                  ●               ●                        
                |                      ●     ●                              
                |                      ●       ●                            
                |                      ●   ●                                
                |                      ●         ●                          
                |                      ● ●                                  
                |                      ●           ●                        
                |                      ●                                    
                |                      ●             ●                      
                |                    ● ●                                    
                |                      ●               ●                    
                |                  ●   ●                                    
                |                      ●                 ●                  
                |                ●     ●                                    
                |                            ●       ●                      
            -50-|                              ●     ●                      
                |                          ●         ●                      
                |                                ●   ●                      
                |                        ●           ●                      
                |                                  ● ●                      
                |                      ●             ●                      
                |                                    ●                      
                |                    ●               ●                      
                |                                    ● ●                    
                |                  ●                 ●                      
                |                                    ●   ●                  
                |                ●                   ●                      
                |                                    ●     ●                
                |              ●                     ●                      
                |                    ●       ●                              
                |                    ●         ●                            
                |                    ●     ●                                
                |                    ●           ●                          
                |                    ●   ●                                  
                |                    ●             ●                        
                |                    ● ●                                    
                |                    ●               ●                      
                |                    ●                                      
                |                    ●                 ●                    
                |                  ● ●                                      
                |                    ●                   ●                  
                |                ●   ●                                      
                |                    ●                     ●                
                |              ●     ●                                      
                |                    ●                       ●              
                |            ●       ●                                      
                |                            ●         ●                    
                |                              ●       ●                    
                |                          ●           ●                    
                |                                ●     ●                    
                |                        ●             ●                    
                |                                  ●   ●                    
                |                      ●               ●                    
                |                                    ● ●                    
                |                    ●                 ●                    
                |                                      ●                    
                |                  ●                   ●                    
                |                                      ● ●                  
                |                ●                     ●                    
                |                                      ●   ●                
                |              ●                       ●                    
                |                                      ●     ●              
                |            ●                         ●                    
                |                                      ●       ●            
                |          ●                           ●                    
           -100-|                  ●         ●                              
                |                  ●           ●                            
                |                  ●       ●                                
                |                  ●             ●                          
                |                  ●     ●                                  
                |                  ●               ●                        
                |                  ●   ●                                    
                |                  ●                 ●                      
                |                  ● ●                                      
                |                  ●                   ●                    
                |                  ●                                        
                |                  ●                     ●                  
                |                ● ●                                        
                |                  ●                       ●                
                |              ●   ●                                        
                |                  ●                         ●              
                |            ●     ●                                        
                |                  ●                           ●            
                |          ●       ●                                        
                |                  ●                             ●          
                |        ●         ●                                        
                |                            ●           ●                  
                |                              ●         ●                  
                |                          ●             ●                  
                |                                ●       ●                  
                |                        ●               ●                  
                |                                  ●     ●                  
                |                      ●                 ●                  
                |                                                           
                |                                    ●   ●                  
                |                    ●                   ●                  
                |                                      ● ●                  
                |                  ●                     ●                  
                |                                        ●                  
                |                ●                       ●                  
                |                                        ● ●                
                |              ●                         ●                  
                |                                        ●   ●              
                |            ●                           ●                  
                |                                        ●     ●            
                |          ●                             ●                  
                |                                        ●       ●          
                |        ●                               ●                  
                |                                        ●         ●        
                |      ●                                 ●                  
                |                ●           ●                              
                |                ●             ●                            
                |                ●         ●                                
                |                ●               ●                          
                |                ●       ●                                  
                |                ●                 ●                        
           -150-|                ●     ●                                    
                |                ●                   ●                      
                |                ●   ●                                      
                |                ●                     ●                    
                |                ● ●                                        
                |                ●                       ●                  
                |                ●                                          
                |                ●                         ●                
                |              ● ●                                          
                |                ●                           ●              
                |            ●   ●                                          
                |                ●                             ●            
                |          ●     ●                                          
                |                ●                               ●          
                |        ●       ●                                          
                |                ●                                 ●        
                |      ●         ●                                          
                |                ●                                   ●      
                |    ●           ●                                          
                |                            ●             ●                
                |                              ●           ●                
                |                          ●               ●                
                |                                ●         ●                
                |                        ●                 ●                
                |                                  ●       ●                
                |                      ●                   ●                
                |                                    ●     ●                
                |                    ●                     ●                
                |                                      ●   ●                
                |                  ●                       ●                
                |                                        ● ●                
                |                ●                         ●                
                |                                          ●                
                |              ●                           ●                
                |                                          ● ●              
                |            ●                             ●                
                |                                          ●   ●            
                |          ●                               ●                
                |                                          ●     ●          
                |        ●                                 ●                
                |                                          ●       ●        
                |      ●                                   ●                
                |                                          ●         ●      
                |    ●                                     ●                
                |                                          ●           ●    
                |  ●                                       ●                
                |              ●             ●                              
                |              ●               ●                            
                |              ●           ●                                
                |              ●                 ●                          
           -200-|              ●         ●                                  
                |              ●                   ●                        
                |              ●       ●                                    
                |              ●                     ●                      
                |              ●     ●                                      
                |              ●                       ●                    
                |              ●   ●                                        
                |              ●                         ●                  
                |              ● ●                                          
                |              ●                           ●                
                |              ●                                            
                |              ●                             ●              
                |            ● ●                                            
                |              ●                               ●            
                |          ●   ●                                            
                |              ●                                 ●          
                |        ●     ●                                            
                |              ●                                   ●        
                |      ●       ●                                            
                |              ●                                     ●      
                |    ●         ●                                            
                |              ●                                       ●    
                |  ●           ●                                            
                |              ●                                         ●  
                |●             ●                                            
                |                            ●               ●              
                |                              ●             ●              
                |                          ●                 ●              
                |                                ●           ●              
                |                        ●                   ●              
                |                                  ●         ●              
                |                      ●                     ●              
                |                                    ●       ●              
                |                    ●                       ●              
                |                                      ●     ●              
                |                  ●                         ●              
                |                                        ●   ●              
                |                ●                           ●              
                |                                          ● ●              
                |              ●                             ●              
                |                                            ●              
                |            ●                               ●              
                |                                            ● ●            
                |          ●                                 ●              
                |                                            ●   ●          
                |        ●                                   ●              
                |                                            ●     ●        
                |      ●                                     ●              
                |                                            ●       ●      
                |    ●                                       ●              
           -250-|                                            ●         ●    
                |  ●                                         ●              
                |                                            ●           ●  
                |●                                           ●              
                |                                            ●             ●
               +------------------------------------------------------------ 
                         |                   |                   |           
                        -10                  0                  10     
    "#]]
    .assert_eq(&actual);
}
