use {
    core::ops::{Add, Div, Mul, Neg, Sub},
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
                    let prime = $working::try_from(self).unwrap();
                    let alfa = prime.mul(8).add(1).sqrt().sub(1).div(2);
                    let bravo = alfa.add(1).mul(alfa).div(2).neg().add(prime);
                    let charlie = $unsigned::try_from(alfa).unwrap().zigzag();
                    let delta = $unsigned::try_from(bravo).unwrap().zigzag();
                    if charlie <= delta {
                        (charlie, delta)
                    } else {
                        (delta, charlie)
                    }
                }
            }

            impl ZugZug for ($signed, $signed) {
                type Output = $unsigned;

                #[inline(always)]
                fn zugzug(self) -> $unsigned {
                    let (lo, hi) = self;
                    debug_assert!(lo <= hi);
                    let (alfa, bravo) = if (lo as $working * 2 - 1).abs() > (hi as $working * 2 - 1).abs() {
                        (lo, hi)
                    } else {
                        (hi, lo)
                    };
                    let charlie = alfa.zigzag() as $working;
                    let delta = bravo.zigzag() as $working;
                    charlie.add(1).mul(charlie).div(2).add(delta) as $unsigned
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
