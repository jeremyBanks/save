use {
    core::ops::{Add, Div, Mul, Sub},
    num_integer::Roots,
};

pub trait ZigZag {
    type Output;

    fn zigzag(self) -> Self::Output;
}

pub trait ZugZug {
    type Output;

    fn zugzug(self) -> Self::Output;
}

macro_rules! impls {
    {$( $signed:ident <-> $unsigned:ident $(using $working:ident)?; )+} => {$(
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

        $(
            impl ZugZug for $unsigned {
                type Output = ($signed, $signed);

                #[inline(always)]
                fn zugzug(self) -> ($signed, $signed) {
                    let this = $working::try_from(self).unwrap();
                    let tick = (((1 + 8 * this).sqrt() - 1) / 2);
                    let tock = (this - tick * (tick + 1) / 2);

                    let alfa = $unsigned::try_from(tick).unwrap().zigzag();
                    let bravo = $unsigned::try_from(tock).unwrap().zigzag();

                    if alfa <= bravo {
                        (alfa, bravo)
                    } else {
                        (bravo, alfa)
                    }
                }
            }

            impl ZugZug for ($signed, $signed) {
                type Output = $unsigned;

                #[inline(always)]
                fn zugzug(self) -> $unsigned {
                    let (lo, hi) = self;
                    debug_assert!(lo <= hi);
                    let lo_magnitude = (lo as $working * 2 - 1).abs();
                    let hi_magnitude = (hi as $working * 2 - 1).abs();

                    let (larger, smaller) = if lo_magnitude > hi_magnitude {
                        (lo, hi)
                    } else {
                        (hi, lo)
                    };

                    let outer = larger.zigzag() as $working;
                    let inner = smaller.zigzag() as $working;

                    outer.add(1).mul(outer).div(2).add(inner) as $unsigned
                }
            }
        )?
    )+};
}

impls! {
       i8 <->    u8 using  i16;
      i16 <->   u16 using  i32;
      i32 <->   u32 using  i64;
      i64 <->   u64 using i128;
     i128 <->  u128;
    isize <-> usize using i128;
}

#[cfg(test)]
mod test {
    use {
        super::*,
        ::{expect_test::expect, plotlib},
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
        let mut existing = std::collections::HashMap::new();
        for i in u8::MIN..=u8::MAX {
            let x = i.zigzag();
            let round_trip = x.zigzag();
            let previous = existing.insert(x, i);
            actual += &format!("     {i:>3}: {x:>4}");
            if let Some(index) = previous {
                actual += &format!("  ERROR: {i:>3} IS A DUPLICATE OF {index:>3}!\n");
            } else if i != round_trip {
                actual += &format!("  ERROR: {i:>3} ROUND TRIPS AS {round_trip:>3}!\n");
            } else if i % 4 == 3 {
                actual += "\n";
            }
        }

        expect![[r#"
                   0:    0       1:    1       2:   -1       3:    2
                   4:   -2       5:    3       6:   -3       7:    4
                   8:   -4       9:    5      10:   -5      11:    6
                  12:   -6      13:    7      14:   -7      15:    8
                  16:   -8      17:    9      18:   -9      19:   10
                  20:  -10      21:   11      22:  -11      23:   12
                  24:  -12      25:   13      26:  -13      27:   14
                  28:  -14      29:   15      30:  -15      31:   16
                  32:  -16      33:   17      34:  -17      35:   18
                  36:  -18      37:   19      38:  -19      39:   20
                  40:  -20      41:   21      42:  -21      43:   22
                  44:  -22      45:   23      46:  -23      47:   24
                  48:  -24      49:   25      50:  -25      51:   26
                  52:  -26      53:   27      54:  -27      55:   28
                  56:  -28      57:   29      58:  -29      59:   30
                  60:  -30      61:   31      62:  -31      63:   32
                  64:  -32      65:   33      66:  -33      67:   34
                  68:  -34      69:   35      70:  -35      71:   36
                  72:  -36      73:   37      74:  -37      75:   38
                  76:  -38      77:   39      78:  -39      79:   40
                  80:  -40      81:   41      82:  -41      83:   42
                  84:  -42      85:   43      86:  -43      87:   44
                  88:  -44      89:   45      90:  -45      91:   46
                  92:  -46      93:   47      94:  -47      95:   48
                  96:  -48      97:   49      98:  -49      99:   50
                 100:  -50     101:   51     102:  -51     103:   52
                 104:  -52     105:   53     106:  -53     107:   54
                 108:  -54     109:   55     110:  -55     111:   56
                 112:  -56     113:   57     114:  -57     115:   58
                 116:  -58     117:   59     118:  -59     119:   60
                 120:  -60     121:   61     122:  -61     123:   62
                 124:  -62     125:   63     126:  -63     127:   64
                 128:  -64     129:   65     130:  -65     131:   66
                 132:  -66     133:   67     134:  -67     135:   68
                 136:  -68     137:   69     138:  -69     139:   70
                 140:  -70     141:   71     142:  -71     143:   72
                 144:  -72     145:   73     146:  -73     147:   74
                 148:  -74     149:   75     150:  -75     151:   76
                 152:  -76     153:   77     154:  -77     155:   78
                 156:  -78     157:   79     158:  -79     159:   80
                 160:  -80     161:   81     162:  -81     163:   82
                 164:  -82     165:   83     166:  -83     167:   84
                 168:  -84     169:   85     170:  -85     171:   86
                 172:  -86     173:   87     174:  -87     175:   88
                 176:  -88     177:   89     178:  -89     179:   90
                 180:  -90     181:   91     182:  -91     183:   92
                 184:  -92     185:   93     186:  -93     187:   94
                 188:  -94     189:   95     190:  -95     191:   96
                 192:  -96     193:   97     194:  -97     195:   98
                 196:  -98     197:   99     198:  -99     199:  100
                 200: -100     201:  101     202: -101     203:  102
                 204: -102     205:  103     206: -103     207:  104
                 208: -104     209:  105     210: -105     211:  106
                 212: -106     213:  107     214: -107     215:  108
                 216: -108     217:  109     218: -109     219:  110
                 220: -110     221:  111     222: -111     223:  112
                 224: -112     225:  113     226: -113     227:  114
                 228: -114     229:  115     230: -115     231:  116
                 232: -116     233:  117     234: -117     235:  118
                 236: -118     237:  119     238: -119     239:  120
                 240: -120     241:  121     242: -121     243:  122
                 244: -122     245:  123     246: -123     247:  124
                 248: -124     249:  125     250: -125     251:  126
                 252: -126     253:  127     254: -127     255: -128
        "#]]
        .assert_eq(&actual);
    }

    #[test]
    fn zigzag_plot() {
        use plotlib::{
            page::Page,
            repr::Plot,
            style::{PointMarker, PointStyle},
            view::ContinuousView,
        };

        let mut actual = String::new();

        let mut view = ContinuousView::new();
        let data = (0..=255u8)
            .enumerate()
            .map(|(i, x): (usize, u8)| (x.zigzag() as f64, -(i as f64)))
            .collect();

        let plot = Plot::new(data).point_style(PointStyle::new().marker(PointMarker::Circle));

        view = view.add(plot);

        actual += Page::single(&view)
            .dimensions(64, 48)
            .to_text()
            .unwrap()
            .trim_end_matches(' ');

        expect![[r#"
                  0-|                              ●                                
                    |                             ●●●                               
                    |                             ● ●●                              
                    |                            ●   ●                              
                    |                           ●●    ●                             
                    |                           ●     ●●                            
                    |                          ●       ●                            
                    |                         ●●        ●                           
                    |                         ●         ●●                          
                -50-|                        ●           ●                          
                    |                       ●●            ●                         
                    |                       ●             ●●                        
                    |                      ●               ●                        
                    |                     ●●                ●                       
                    |                     ●                 ●●                      
                    |                    ●                   ●                      
                    |                   ●●                    ●                     
                    |                   ●                     ●●                    
                    |                  ●                       ●                    
               -100-|                 ●●                        ●                   
                    |                 ●                         ●●                  
                    |                ●                           ●                  
                    |               ●●                            ●                 
                    |               ●                             ●●                
                    |              ●                               ●                
                    |             ●●                                ●               
                    |             ●                                 ●●              
                    |            ●                                   ●              
               -150-|           ●●                                    ●             
                    |           ●                                     ●●            
                    |          ●                                       ●            
                    |         ●●                                        ●           
                    |         ●                                         ●●          
                    |        ●                                           ●          
                    |       ●●                                            ●         
                    |       ●                                             ●●        
                    |      ●                                               ●        
                    |     ●●                                                ●       
               -200-|     ●                                                 ●●      
                    |    ●                                                   ●      
                    |   ●●                                                    ●     
                    |   ●                                                     ●●    
                    |  ●                                                       ●    
                    | ●●                                                        ●   
                    | ●                                                         ●●  
                    |●                                                           ●  
                    |●                                                            ● 
               -250-|                                                             ●●
                   +---------------------------------------------------------------- 
                          |            |           |            |           |        
                        -100          -50          0           50          100      
        "#]]
        .assert_eq(&actual);
    }

    #[test]
    fn zugzug_round_trip() {
        for uint in u8::MIN..=u8::MAX {
            assert_eq!(uint, uint.zugzug().zugzug());
        }

        for uint in u16::MIN..=u16::MAX {
            assert_eq!(uint, uint.zugzug().zugzug());
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
        let mut existing = std::collections::HashMap::new();
        for i in u8::MIN..=u8::MAX {
            let (x, y) = i.zugzug();
            let round_trip = (x, y).zugzug();
            let previous = existing.insert((x, y), i);
            actual += &format!("    {i:>3}: ({x:>3}, {y:>3})");
            if let Some(index) = previous {
                actual += &format!("  ERROR: {i:>3} IS A DUPLICATE OF {index:>3}!\n");
            } else if round_trip != i {
                actual += &format!("  ERROR: {i:>3} ROUND TRIPS TO {round_trip:>3}\n");
            } else if i % 4 == 3 {
                actual += "\n";
            }
        }

        expect![[r#"
                  0: (  0,   0)      1: (  0,   1)      2: (  1,   1)      3: ( -1,   0)
                  4: ( -1,   1)      5: ( -1,  -1)      6: (  0,   2)      7: (  1,   2)
                  8: ( -1,   2)      9: (  2,   2)     10: ( -2,   0)     11: ( -2,   1)
                 12: ( -2,  -1)     13: ( -2,   2)     14: ( -2,  -2)     15: (  0,   3)
                 16: (  1,   3)     17: ( -1,   3)     18: (  2,   3)     19: ( -2,   3)
                 20: (  3,   3)     21: ( -3,   0)     22: ( -3,   1)     23: ( -3,  -1)
                 24: ( -3,   2)     25: ( -3,  -2)     26: ( -3,   3)     27: ( -3,  -3)
                 28: (  0,   4)     29: (  1,   4)     30: ( -1,   4)     31: (  2,   4)
                 32: ( -2,   4)     33: (  3,   4)     34: ( -3,   4)     35: (  4,   4)
                 36: ( -4,   0)     37: ( -4,   1)     38: ( -4,  -1)     39: ( -4,   2)
                 40: ( -4,  -2)     41: ( -4,   3)     42: ( -4,  -3)     43: ( -4,   4)
                 44: ( -4,  -4)     45: (  0,   5)     46: (  1,   5)     47: ( -1,   5)
                 48: (  2,   5)     49: ( -2,   5)     50: (  3,   5)     51: ( -3,   5)
                 52: (  4,   5)     53: ( -4,   5)     54: (  5,   5)     55: ( -5,   0)
                 56: ( -5,   1)     57: ( -5,  -1)     58: ( -5,   2)     59: ( -5,  -2)
                 60: ( -5,   3)     61: ( -5,  -3)     62: ( -5,   4)     63: ( -5,  -4)
                 64: ( -5,   5)     65: ( -5,  -5)     66: (  0,   6)     67: (  1,   6)
                 68: ( -1,   6)     69: (  2,   6)     70: ( -2,   6)     71: (  3,   6)
                 72: ( -3,   6)     73: (  4,   6)     74: ( -4,   6)     75: (  5,   6)
                 76: ( -5,   6)     77: (  6,   6)     78: ( -6,   0)     79: ( -6,   1)
                 80: ( -6,  -1)     81: ( -6,   2)     82: ( -6,  -2)     83: ( -6,   3)
                 84: ( -6,  -3)     85: ( -6,   4)     86: ( -6,  -4)     87: ( -6,   5)
                 88: ( -6,  -5)     89: ( -6,   6)     90: ( -6,  -6)     91: (  0,   7)
                 92: (  1,   7)     93: ( -1,   7)     94: (  2,   7)     95: ( -2,   7)
                 96: (  3,   7)     97: ( -3,   7)     98: (  4,   7)     99: ( -4,   7)
                100: (  5,   7)    101: ( -5,   7)    102: (  6,   7)    103: ( -6,   7)
                104: (  7,   7)    105: ( -7,   0)    106: ( -7,   1)    107: ( -7,  -1)
                108: ( -7,   2)    109: ( -7,  -2)    110: ( -7,   3)    111: ( -7,  -3)
                112: ( -7,   4)    113: ( -7,  -4)    114: ( -7,   5)    115: ( -7,  -5)
                116: ( -7,   6)    117: ( -7,  -6)    118: ( -7,   7)    119: ( -7,  -7)
                120: (  0,   8)    121: (  1,   8)    122: ( -1,   8)    123: (  2,   8)
                124: ( -2,   8)    125: (  3,   8)    126: ( -3,   8)    127: (  4,   8)
                128: ( -4,   8)    129: (  5,   8)    130: ( -5,   8)    131: (  6,   8)
                132: ( -6,   8)    133: (  7,   8)    134: ( -7,   8)    135: (  8,   8)
                136: ( -8,   0)    137: ( -8,   1)    138: ( -8,  -1)    139: ( -8,   2)
                140: ( -8,  -2)    141: ( -8,   3)    142: ( -8,  -3)    143: ( -8,   4)
                144: ( -8,  -4)    145: ( -8,   5)    146: ( -8,  -5)    147: ( -8,   6)
                148: ( -8,  -6)    149: ( -8,   7)    150: ( -8,  -7)    151: ( -8,   8)
                152: ( -8,  -8)    153: (  0,   9)    154: (  1,   9)    155: ( -1,   9)
                156: (  2,   9)    157: ( -2,   9)    158: (  3,   9)    159: ( -3,   9)
                160: (  4,   9)    161: ( -4,   9)    162: (  5,   9)    163: ( -5,   9)
                164: (  6,   9)    165: ( -6,   9)    166: (  7,   9)    167: ( -7,   9)
                168: (  8,   9)    169: ( -8,   9)    170: (  9,   9)    171: ( -9,   0)
                172: ( -9,   1)    173: ( -9,  -1)    174: ( -9,   2)    175: ( -9,  -2)
                176: ( -9,   3)    177: ( -9,  -3)    178: ( -9,   4)    179: ( -9,  -4)
                180: ( -9,   5)    181: ( -9,  -5)    182: ( -9,   6)    183: ( -9,  -6)
                184: ( -9,   7)    185: ( -9,  -7)    186: ( -9,   8)    187: ( -9,  -8)
                188: ( -9,   9)    189: ( -9,  -9)    190: (  0,  10)    191: (  1,  10)
                192: ( -1,  10)    193: (  2,  10)    194: ( -2,  10)    195: (  3,  10)
                196: ( -3,  10)    197: (  4,  10)    198: ( -4,  10)    199: (  5,  10)
                200: ( -5,  10)    201: (  6,  10)    202: ( -6,  10)    203: (  7,  10)
                204: ( -7,  10)    205: (  8,  10)    206: ( -8,  10)    207: (  9,  10)
                208: ( -9,  10)    209: ( 10,  10)    210: (-10,   0)    211: (-10,   1)
                212: (-10,  -1)    213: (-10,   2)    214: (-10,  -2)    215: (-10,   3)
                216: (-10,  -3)    217: (-10,   4)    218: (-10,  -4)    219: (-10,   5)
                220: (-10,  -5)    221: (-10,   6)    222: (-10,  -6)    223: (-10,   7)
                224: (-10,  -7)    225: (-10,   8)    226: (-10,  -8)    227: (-10,   9)
                228: (-10,  -9)    229: (-10,  10)    230: (-10, -10)    231: (  0,  11)
                232: (  1,  11)    233: ( -1,  11)    234: (  2,  11)    235: ( -2,  11)
                236: (  3,  11)    237: ( -3,  11)    238: (  4,  11)    239: ( -4,  11)
                240: (  5,  11)    241: ( -5,  11)    242: (  6,  11)    243: ( -6,  11)
                244: (  7,  11)    245: ( -7,  11)    246: (  8,  11)    247: ( -8,  11)
                248: (  9,  11)    249: ( -9,  11)    250: ( 10,  11)    251: (-10,  11)
                252: ( 11,  11)    253: (-11,   0)    254: (-11,   1)    255: (-11,  -1)
        "#]]
        .assert_eq(&actual);
    }

    #[test]
    fn zugzug_plot() {
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
            .dimensions(64, 48)
            .to_text()
            .unwrap()
            .trim_end_matches(' ');

        expect![[r#"
                  0-|                              ●  ●                             
                    |                           ●  ●  ●  ●                          
                    |                        ●  ●  ●  ●  ●                          
                    |                        ●  ●  ●  ●  ●  ●                       
                    |                     ●  ●  ●  ●  ●     ●                       
                    |                     ●  ●     ●  ●  ●  ●  ●                    
                    |                     ●  ●  ●        ●  ●  ●                    
                    |                  ●        ●  ●  ●  ●     ●                    
                    |                  ●  ●  ●     ●        ●  ●  ●                 
                -50-|                        ●  ●     ●  ●  ●     ●                 
                    |               ●  ●  ●        ●           ●  ●                 
                    |               ●     ●  ●  ●     ●  ●  ●                       
                    |               ●  ●           ●           ●  ● ●               
                    |                        ●  ●     ●  ●  ●       ●               
                    |               ●  ●  ●                    ●  ● ●               
                    |             ●          ●  ●  ●  ●  ●                          
                    |             ●    ●  ●                 ●  ●  ●                 
                    |             ● ●              ●  ●             ●  ●            
                    |                     ●  ●  ●        ●  ●  ●       ●            
               -100-|             ● ●  ●                          ● ●  ●            
                    |          ●                ●  ●  ●  ●             ●            
                    |          ●       ●  ●  ●              ●  ●  ●                 
                    |          ●  ● ●                               ●  ●            
                    |                        ●  ●  ●  ●  ●                ●         
                    |               ●  ●  ●                 ●  ●  ●       ●         
                    |          ●  ●                                 ●  ●  ●         
                    |       ●                ●  ●  ●  ●  ●                          
                    |       ●       ●  ●  ●                 ●  ●  ●                 
               -150-|       ●  ●  ●                                 ●  ●  ●         
                    |       ●                   ●  ●  ●  ●                   ●      
                    |                  ●  ●  ●              ●  ●  ●          ●      
                    |          ●  ● ●                               ●  ●     ●      
                    |    ●  ●                      ●  ●                   ●  ●      
                    |    ●                ●  ●  ●        ●  ●                       
                    |    ●        ● ●  ●                       ●  ● ●               
                    |    ●  ●  ●                                       ●  ●  ●      
                    |    ●                      ●  ●  ●  ●                      ●   
                    |                  ●  ●  ●              ●  ●  ●             ●   
               -200-|          ●  ● ●                               ●  ●        ●   
                    |    ●  ●                                             ●  ●  ●   
                    | ●                      ●  ●  ●  ●  ●  ●                       
                    | ●             ●  ●  ●                    ●  ●                 
                    | ●        ●  ●                                 ●  ●  ●         
                    | ●  ●  ●                      ●                         ●  ●  ●
                    |                        ●  ●     ●  ●  ●                      ●
                    |               ●  ●  ●                    ●  ●                ●
                    |       ●  ●  ●                                 ●  ●  ●        ●
               -250-| ●  ●                                                   ●  ●  ●
                   +---------------------------------------------------------------- 
                      |             |              |              |             |    
                     -10           -5              0              5            10    
        "#]]
        .assert_eq(&actual);
    }
}
