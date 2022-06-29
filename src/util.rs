pub trait ZigZag: Copy + ::core::fmt::Debug {
    type Output;
    fn zigzag(self) -> Self::Output;
}

macro_rules! impl_for {
    { $signed:ident <-> $unsigned:ident } => {
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
    };
}

impl_for! {    i8 <->    u8 }
impl_for! {   i16 <->   u16 }
impl_for! {   i32 <->   u32 }
impl_for! {   i64 <->   u64 }
impl_for! {  i128 <->  u128 }
impl_for! { isize <-> usize }

#[test]
fn test_zigzag_round_trip() {
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
fn test_zigzag_known_values() {
    assert_eq!(0_u8.zigzag(), 0_i8);
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
}
