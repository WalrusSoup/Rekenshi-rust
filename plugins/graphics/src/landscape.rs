//! Conservative placement policy and affine packing, independent of native ABI.
#![forbid(unsafe_code)]

/// Match the regular builder's positive uniform-scale transform. Other scale
/// shapes require separately verified normal/winding behavior before inclusion.
pub(crate) fn affine(
    position: [f32; 3],
    quaternion_wxyz: [f32; 4],
    scale: [f32; 3],
    color: [f32; 4],
    origin: [f32; 3],
) -> Option<[f32; 12]> {
    if !position
        .iter()
        .chain(&quaternion_wxyz)
        .chain(&scale)
        .chain(&color)
        .chain(&origin)
        .all(|v| v.is_finite())
        || color != [1.; 4]
        || scale[0] <= 0.
        || scale[0] != scale[1]
        || scale[0] != scale[2]
        || (quaternion_wxyz.iter().map(|v| v * v).sum::<f32>() - 1.).abs() > 0.0001
    {
        return None;
    }
    let [w, x, y, z] = quaternion_wxyz;
    let s = scale[0];
    let rows = [
        (1. - 2. * (y * y + z * z)) * s,
        2. * (x * y - z * w) * s,
        2. * (x * z + y * w) * s,
        position[0] - origin[0],
        2. * (x * y + z * w) * s,
        (1. - 2. * (x * x + z * z)) * s,
        2. * (y * z - x * w) * s,
        position[1] - origin[1],
        2. * (x * z - y * w) * s,
        2. * (y * z + x * w) * s,
        (1. - 2. * (x * x + y * y)) * s,
        position[2] - origin[2],
    ];
    rows.iter().all(|v| v.is_finite()).then_some(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_wxyz_rotation_and_batch_origin_compose() {
        let rows = affine([12., -2., 7.], [0.5; 4], [2.; 3], [1.; 4], [10., -3., 9.]).unwrap();
        assert_eq!(rows, [0., 0., 2., 2., 2., 0., 0., 1., 0., 2., 0., -2.]);
    }

    #[test]
    fn unsupported_scale_color_and_invalid_quaternions_fall_back() {
        for scale in [[1., 2., 1.], [-1.; 3], [0.; 3], [f32::NAN; 3]] {
            assert!(affine([0.; 3], [1., 0., 0., 0.], scale, [1.; 4], [0.; 3]).is_none());
        }
        for quaternion in [[0.; 4], [1.; 4], [f32::INFINITY; 4]] {
            assert!(affine([0.; 3], quaternion, [1.; 3], [1.; 4], [0.; 3]).is_none());
        }
        assert!(affine([0.; 3], [1., 0., 0., 0.], [1.; 3], [0.5; 4], [0.; 3]).is_none());
    }
}
