

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
                debug_assert!(lo <= hi);
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
fn zugzug_round_trip() {
    for uint in u8::MIN..=u8::MAX {
        assert_eq!(uint, uint.zugzug().zugzug());
    }

    for uint in u16::MIN..=u16::MAX {
        assert_eq!(uint, uint.zugzug().zugzug());
    }
}

#[test]
fn assert_zugzug_known_values() {
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
