use bevy::math::Vec3;

pub fn limit(mut vec: Vec3, max: f32) -> Vec3 {
    let mag_sq = vec.x * vec.x + vec.y * vec.y + vec.z * vec.z;
    if mag_sq > max * max {
        vec = (vec / mag_sq.sqrt()) * max;
    }
    return vec;
}

pub fn set_mag(vec: Vec3, n: f32) -> Vec3 {
    vec.normalize() * n
}

pub fn dist(vec_a: Vec3, vec_b: Vec3) -> f32 {
    ((vec_b.x - vec_a.x) * (vec_b.x - vec_a.x) + (vec_b.y - vec_a.y) * (vec_b.y - vec_a.y)).sqrt()
}

pub fn line_line_intersection(a1: Vec3, a2: Vec3, b1: Vec3, b2: Vec3) -> Result<Vec3, ()> {
    let x1 = a1.x;
    let y1 = a1.y;
    let x2 = a2.x;
    let y2 = a2.y;

    let x3 = b1.x;
    let y3 = b1.y;
    let x4 = b2.x;
    let y4 = b2.y;

    let den = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if den == 0.0 {
        return Err(());
    }

    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / den;
    let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / den;

    if t > 0.0 && t < 1.0 && u > 1.0 {
        return Ok(Vec3::new(x1 + t * (x2 - x1), y1 + t * (y2 - y1), 0.0));
    }

    Err(())
}
