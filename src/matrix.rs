use crate::types::{Mat3, Vec3};

/// Calculates the dot-product of 3x3 matrix m with the 3-element vector v.
#[inline]
pub fn mat3_dot(m: Mat3, v: Vec3) -> Vec3 {
    let [m00, m01, m02, m10, m11, m12, m20, m21, m22] = m;
    let [v0, v1, v2] = v;

    // Dot-products implemented with FMADD to minimize loss of precision.
    [
        m00.mul_add(v0, m01.mul_add(v1, m02 * v2)),
        m10.mul_add(v0, m11.mul_add(v1, m12 * v2)),
        m20.mul_add(v0, m21.mul_add(v1, m22 * v2)),
    ]
}
