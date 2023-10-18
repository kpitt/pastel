use crate::helper::Fraction;

pub trait ColorSpace {
    fn mix(self, other: Self, fraction: Fraction) -> Self;
}
