use std::fmt::{self, Display};

use crate::types::Scalar;

/// Like `%`, but always positive.
pub fn mod_positive(x: Scalar, y: Scalar) -> Scalar {
    (x % y + y) % y
}

/// Trim a number such that it fits into the range [lower, upper].
pub fn clamp(lower: Scalar, upper: Scalar, x: Scalar) -> Scalar {
    Scalar::max(Scalar::min(upper, x), lower)
}

#[derive(Debug, Clone, Copy)]
pub struct Fraction {
    f: Scalar,
}

impl Fraction {
    pub fn from(s: Scalar) -> Self {
        Fraction {
            f: clamp(0.0, 1.0, s),
        }
    }

    pub fn value(self) -> Scalar {
        self.f
    }
}

// `format!`-style format strings only allow specifying a fixed floating
// point precision, e.g. `{:.3}` to print 3 decimal places. This always
// displays trailing zeroes, while web colors generally omit them. For
// example, we'd prefer to print `0.5` as `0.5` instead of `0.500`.
//
// Note that this will round using omitted decimal places:
//
//     MaxPrecision::wrap(3, 0.5004) //=> 0.500
//     MaxPrecision::wrap(3, 0.5005) //=> 0.501
//
pub struct MaxPrecision {
    precision: u32,
    inner: f64,
}

impl MaxPrecision {
    pub fn wrap(precision: u32, inner: f64) -> Self {
        Self { precision, inner }
    }
}

impl Display for MaxPrecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pow_10 = 10u32.pow(self.precision) as f64;
        let rounded = (self.inner * pow_10).round() / pow_10;
        write!(f, "{}", rounded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_mod_positive() {
        assert_relative_eq!(0.5, mod_positive(2.9, 2.4));
        assert_relative_eq!(1.7, mod_positive(-0.3, 2.0));
    }

    #[test]
    fn test_max_precision() {
        assert_eq!(format!("{}", MaxPrecision::wrap(3, 0.5)), "0.5");
        assert_eq!(format!("{}", MaxPrecision::wrap(3, 0.51)), "0.51");
        assert_eq!(format!("{}", MaxPrecision::wrap(3, 0.512)), "0.512");
        assert_eq!(format!("{}", MaxPrecision::wrap(3, 0.5124)), "0.512");
        assert_eq!(format!("{}", MaxPrecision::wrap(3, 0.5125)), "0.513");
    }
}
