use std::f64::consts::PI;

pub fn abc_to_alpha_beta_0(a: f64, b: f64, c: f64) -> (f64, f64, f64) {
    let alpha = (2.0 / 3.0) * (a - b / 2.0 - c / 2.0);
    let beta = (2.0 / 3.0) * ((3.0_f64).sqrt() * (b - c) / 2.0);
    let z = (2.0 / 3.0) * ((a + b + c) / 2.0);
    (alpha, beta, z)
}

pub fn alpha_beta_0_to_abc(alpha: f64, beta: f64, z: f64) -> (f64, f64, f64) {
    let a = alpha + z;
    let b = -alpha / 2.0 + beta * (3.0_f64).sqrt() / 2.0 + z;
    let c = -alpha / 2.0 - beta * (3.0_f64).sqrt() / 2.0 + z;
    (a, b, c)
}

pub fn abc_to_dq0(a: f64, b: f64, c: f64, wt: f64, delta: f64) -> (f64, f64, f64) {
    let angle = wt + delta;
    let d = (2.0 / 3.0) * (a * angle.sin() + b * (angle - 2.0 * PI / 3.0).sin() + c * (angle + 2.0 * PI / 3.0).sin());
    let q = (2.0 / 3.0) * (a * angle.cos() + b * (angle - 2.0 * PI / 3.0).cos() + c * (angle + 2.0 * PI / 3.0).cos());
    let z = (2.0 / 3.0) * (a + b + c) / 2.0;
    (d, q, z)
}

pub fn dq0_to_abc(d: f64, q: f64, z: f64, wt: f64, delta: f64) -> (f64, f64, f64) {
    let a = d * (wt + delta).sin() + q * (wt + delta).cos() + z;
    let b = d * (wt - 2.0 * PI / 3.0 + delta).sin() + q * (wt - 2.0 * PI / 3.0 + delta).cos() + z;
    let c = d * (wt + 2.0 * PI / 3.0 + delta).sin() + q * (wt + 2.0 * PI / 3.0 + delta).cos() + z;
    (a, b, c)
}

pub fn alpha_beta_0_to_dq0(alpha: f64, beta: f64, zero: f64, wt: f64, delta: f64) -> (f64, f64, f64) {
    let d = alpha * (wt + delta).sin() - beta * (wt + delta).cos();
    let q = alpha * (wt + delta).cos() + beta * (wt + delta).sin();
    let z = zero;
    (d, q, z)
}