use {
    core::ops::{Add, Div, Mul, Sub},
    num_integer::Roots,
};

#[cfg(test)]
mod test;

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
