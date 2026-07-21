#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod transforms;

use eframe::egui;
use egui::Color32;
use egui_plot::{Arrows, HLine, Line, LineStyle, Plot, PlotPoints, PlotUi, VLine};
use std::f64::consts::{PI, SQRT_2};

const COL_A: Color32 = Color32::from_rgb(0xe5, 0x39, 0x35);
const COL_B: Color32 = Color32::from_rgb(0x43, 0xa0, 0x47);
const COL_C: Color32 = Color32::from_rgb(0x1e, 0x88, 0xe5);
const COL_ALPHA: Color32 = Color32::from_rgb(0xfb, 0x8c, 0x00);
const COL_BETA: Color32 = Color32::from_rgb(0x8e, 0x24, 0xaa);
const COL_ZERO: Color32 = Color32::from_rgb(0x8d, 0x6e, 0x63);
const COL_D: Color32 = Color32::from_rgb(0x00, 0xac, 0xc1);
const COL_Q: Color32 = Color32::from_rgb(0xf4, 0x51, 0x1e);
const COL_VEC: Color32 = Color32::from_rgb(0xd8, 0x1b, 0x60);
const COL_CURSOR: Color32 = Color32::from_rgb(0xff, 0xca, 0x28);

const RADIAL_WIDTH: f32 = 170.0;
const ROW_HEIGHT: f32 = 150.0;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Clarke and Park Transformations",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    mag_a: f64,
    mag_b: f64,
    mag_c: f64,
    ang_a: f64,
    ang_b: f64,
    ang_c: f64,
    freq: f64,
    delta: f64,
    ref_dfreq: f64,
    harm5: f64,
    harm7: f64,
    is_rms: bool,
    power_invariant: bool,
    show_dq_axes: bool,
    show_construction: bool,
    fault: String,
    sag: f64,
    anim_time: f64,
    stop_time: f64,
    anim_duration: f64,
    paused: bool,
    cursor_time: Option<f64>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            mag_a: 1.0,
            mag_b: 1.0,
            mag_c: 1.0,
            ang_a: 0.0,
            ang_b: -120.0,
            ang_c: 120.0,
            freq: 50.0,
            delta: 0.0,
            ref_dfreq: 0.0,
            harm5: 0.0,
            harm7: 0.0,
            is_rms: true,
            power_invariant: false,
            show_dq_axes: true,
            show_construction: true,
            fault: "None".to_string(),
            sag: 50.0,
            anim_time: 0.0,
            stop_time: 0.1667,
            anim_duration: 1.0,
            paused: false,
            cursor_time: None,
        }
    }
}

/// Full pipeline state at one instant, used by the radial views, the phasor
/// trails and the numeric readout.
struct Snapshot {
    sig: (f64, f64, f64),
    quad: (f64, f64, f64),
    alpha: f64,
    beta: f64,
    zero: f64,
    d: f64,
    q: f64,
    rec_sig: (f64, f64, f64),
    rec_quad: (f64, f64, f64),
    /// Park frame angle theta = wt_ref + delta at this instant.
    theta: f64,
}

impl MyApp {
    /// Angle of the Park reference frame, which may be de-tuned from the
    /// signal frequency to demonstrate slip.
    fn park_wt(&self, t: f64) -> f64 {
        2.0 * PI * (self.freq + self.ref_dfreq) * t
    }

    /// Display scale factors (alpha-beta-dq, zero) for the selected Clarke
    /// convention: 1 for amplitude-invariant, sqrt(3/2) / sqrt(3) on top of
    /// the internal amplitude-invariant pipeline for power-invariant.
    fn scale_factors(&self) -> (f64, f64) {
        if self.power_invariant {
            ((1.5_f64).sqrt(), (3.0_f64).sqrt())
        } else {
            (1.0, 1.0)
        }
    }

    /// Faulted instantaneous phase values at time `t`. A `phase_shift` of 0
    /// gives the signal itself; PI/2 gives the quadrature component, so each
    /// phasor (quadrature, signal) stays consistent through the fault window.
    fn phases_at(&self, t: f64, phase_shift: f64) -> (f64, f64, f64) {
        let wt = 2.0 * PI * self.freq * t;
        let mult = if self.is_rms { SQRT_2 } else { 1.0 };
        let h5 = self.harm5 / 100.0;
        let h7 = self.harm7 / 100.0;
        let wave = |ang_deg: f64, mag: f64| {
            let th = wt + ang_deg.to_radians();
            mult * mag
                * ((th + phase_shift).sin()
                    + h5 * (5.0 * th + phase_shift).sin()
                    + h7 * (7.0 * th + phase_shift).sin())
        };
        let mut p_a = wave(self.ang_a, self.mag_a);
        let mut p_b = wave(self.ang_b, self.mag_b);
        let mut p_c = wave(self.ang_c, self.mag_c);

        if t >= self.stop_time / 4.0 && t <= 3.0 * self.stop_time / 4.0 {
            match self.fault.as_str() {
                "Monophasic A" => p_a = 0.0,
                "Monophasic B" => p_b = 0.0,
                "Monophasic C" => p_c = 0.0,
                "Two-phase A-B-ground" => { p_a = 0.0; p_b = 0.0; }
                "Two-phase A-B" => {
                    // Ungrounded phase-to-phase fault: both phases collapse to
                    // their common (mean) potential.
                    let v = (p_a + p_b) / 2.0;
                    p_a = v;
                    p_b = v;
                }
                "Two-phase B-C-ground" => { p_b = 0.0; p_c = 0.0; }
                "Two-phase B-C" => {
                    let v = (p_b + p_c) / 2.0;
                    p_b = v;
                    p_c = v;
                }
                "Two-phase C-A-ground" => { p_c = 0.0; p_a = 0.0; }
                "Two-phase C-A" => {
                    let v = (p_c + p_a) / 2.0;
                    p_c = v;
                    p_a = v;
                }
                "Three-phase-ground" => { p_c = 0.0; p_b = 0.0; p_a = 0.0; }
                "Three-phase" => {
                    let v = (p_a + p_b + p_c) / 3.0;
                    p_a = v;
                    p_b = v;
                    p_c = v;
                }
                "Sag A" => p_a = p_a * self.sag / 100.0,
                "Sag AB" => { p_a = p_a * self.sag / 100.0; p_b = p_b * self.sag / 100.0; }
                "Sag ABC" => {
                    p_a = p_a * self.sag / 100.0;
                    p_b = p_b * self.sag / 100.0;
                    p_c = p_c * self.sag / 100.0;
                }
                _ => {}
            }
        }
        (p_a, p_b, p_c)
    }

    fn snapshot(&self, t: f64) -> Snapshot {
        let delta_rad = self.delta.to_radians();
        let wt_ref = self.park_wt(t);
        let sig = self.phases_at(t, 0.0);
        let quad = self.phases_at(t, PI / 2.0);
        let (alpha, beta, zero) = transforms::abc_to_alpha_beta_0(sig.0, sig.1, sig.2);
        let (d, q, _) = transforms::alpha_beta_0_to_dq0(alpha, beta, zero, wt_ref, delta_rad);
        let rec_sig = transforms::dq0_to_abc(d, q, zero, wt_ref, delta_rad);
        // Quadrature components pushed through the same round trip so the
        // reconstructed phasors stay well-defined during faults.
        let (cd, cq, cz) = transforms::abc_to_dq0(quad.0, quad.1, quad.2, wt_ref, delta_rad);
        let rec_quad = transforms::dq0_to_abc(cd, cq, cz, wt_ref, delta_rad);
        Snapshot {
            sig,
            quad,
            alpha,
            beta,
            zero,
            d,
            q,
            rec_sig,
            rec_quad,
            theta: wt_ref + delta_rad,
        }
    }
}

fn circle_line(r: f64) -> Line {
    let pts: Vec<[f64; 2]> = (0..=64)
        .map(|i| {
            let a = 2.0 * PI * i as f64 / 64.0;
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    Line::new(PlotPoints::new(pts)).color(Color32::from_gray(96))
}

fn arrow(from: [f64; 2], to: [f64; 2], color: Color32) -> Arrows {
    Arrows::new(PlotPoints::new(vec![from]), PlotPoints::new(vec![to])).color(color)
}

fn phasor(tip: [f64; 2], color: Color32) -> Arrows {
    arrow([0.0, 0.0], tip, color)
}

fn axis_line(dir: [f64; 2], r: f64, color: Color32) -> Line {
    Line::new(PlotPoints::new(vec![
        [-r * dir[0], -r * dir[1]],
        [r * dir[0], r * dir[1]],
    ]))
    .color(color)
}

fn radial_plot(id: &str, r: f64) -> Plot<'_> {
    Plot::new(id)
        .width(RADIAL_WIDTH)
        .height(ROW_HEIGHT)
        .data_aspect(1.0)
        .include_x(-r)
        .include_x(r)
        .include_y(-r)
        .include_y(r)
        .show_axes([false, false])
        .show_x(false)
        .show_y(false)
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("controls").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Parameters");
                ui.add(egui::Slider::new(&mut self.mag_a, 0.0..=10.0).text("Mag A"));
                ui.add(egui::Slider::new(&mut self.ang_a, -180.0..=180.0).text("Ang A"));
                ui.add(egui::Slider::new(&mut self.mag_b, 0.0..=10.0).text("Mag B"));
                ui.add(egui::Slider::new(&mut self.ang_b, -180.0..=180.0).text("Ang B"));
                ui.add(egui::Slider::new(&mut self.mag_c, 0.0..=10.0).text("Mag C"));
                ui.add(egui::Slider::new(&mut self.ang_c, -180.0..=180.0).text("Ang C"));
                ui.add(egui::Slider::new(&mut self.freq, 1.0..=120.0).text("Freq"));
                ui.add(egui::Slider::new(&mut self.stop_time, 0.01..=1.0).text("Stop Time (s)"));
                ui.add(egui::Slider::new(&mut self.anim_duration, 0.1..=5.0).text("Anim Speed"));

                ui.separator();
                ui.label("Park reference frame");
                ui.add(egui::Slider::new(&mut self.delta, -180.0..=180.0).text("Frame δ (°)"));
                ui.add(egui::Slider::new(&mut self.ref_dfreq, -30.0..=30.0).text("Frame Δf (Hz)"));

                ui.separator();
                ui.label("Harmonics");
                ui.add(egui::Slider::new(&mut self.harm5, 0.0..=50.0).text("5th harm (%)"));
                ui.add(egui::Slider::new(&mut self.harm7, 0.0..=50.0).text("7th harm (%)"));

                ui.separator();
                ui.checkbox(&mut self.is_rms, "RMS values");
                ui.checkbox(&mut self.power_invariant, "Power-invariant (√(2/3))");
                ui.checkbox(&mut self.show_dq_axes, "Show dq axes (αβ view)");
                ui.checkbox(&mut self.show_construction, "Show αβ construction");

                ui.add(egui::Slider::new(&mut self.sag, 0.0..=100.0).text("Sag (%)"));
                egui::ComboBox::from_label("Fault")
                    .selected_text(&self.fault)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.fault, "None".to_string(), "None");
                        ui.selectable_value(&mut self.fault, "Monophasic A".to_string(), "Monophasic A");
                        ui.selectable_value(&mut self.fault, "Monophasic B".to_string(), "Monophasic B");
                        ui.selectable_value(&mut self.fault, "Monophasic C".to_string(), "Monophasic C");
                        ui.selectable_value(&mut self.fault, "Two-phase A-B-ground".to_string(), "Two-phase A-B-ground");
                        ui.selectable_value(&mut self.fault, "Two-phase A-B".to_string(), "Two-phase A-B");
                        ui.selectable_value(&mut self.fault, "Two-phase B-C-ground".to_string(), "Two-phase B-C-ground");
                        ui.selectable_value(&mut self.fault, "Two-phase B-C".to_string(), "Two-phase B-C");
                        ui.selectable_value(&mut self.fault, "Two-phase C-A-ground".to_string(), "Two-phase C-A-ground");
                        ui.selectable_value(&mut self.fault, "Two-phase C-A".to_string(), "Two-phase C-A");
                        ui.selectable_value(&mut self.fault, "Three-phase-ground".to_string(), "Three-phase-ground");
                        ui.selectable_value(&mut self.fault, "Three-phase".to_string(), "Three-phase");
                        ui.selectable_value(&mut self.fault, "Sag A".to_string(), "Sag A");
                        ui.selectable_value(&mut self.fault, "Sag AB".to_string(), "Sag AB");
                        ui.selectable_value(&mut self.fault, "Sag ABC".to_string(), "Sag ABC");
                    });

                ui.horizontal(|ui| {
                    if ui.button("Restart Animation").clicked() {
                        self.anim_time = 0.0;
                    }
                    let pause_label = if self.paused { "Resume" } else { "Pause" };
                    if ui.button(pause_label).clicked() {
                        self.paused = !self.paused;
                        ctx.request_repaint();
                    }
                });

                ui.separator();
                match self.cursor_time {
                    Some(tc) => {
                        ui.label(format!("Inspecting t = {:.4} s", tc));
                        if ui.button("Follow Animation").clicked() {
                            self.cursor_time = None;
                        }
                    }
                    None => {
                        ui.label("Click or drag on a graph\nto inspect an instant");
                    }
                }

                let t_view = self.cursor_time.map_or(self.anim_time, |c| c.min(self.anim_time));
                let snap = self.snapshot(t_view);
                let (k_ab, k_z) = self.scale_factors();
                let mag = (snap.alpha.hypot(snap.beta)) * k_ab;
                let ang = snap.beta.atan2(snap.alpha).to_degrees();
                let theta = ((snap.theta.to_degrees() % 360.0) + 360.0) % 360.0;
                egui::Grid::new("readout").num_columns(2).striped(true).show(ui, |ui| {
                    let row = |ui: &mut egui::Ui, name: &str, val: String| {
                        ui.label(name);
                        ui.label(val);
                        ui.end_row();
                    };
                    row(ui, "t", format!("{:.4} s", t_view));
                    row(ui, "A", format!("{:+.3}", snap.sig.0));
                    row(ui, "B", format!("{:+.3}", snap.sig.1));
                    row(ui, "C", format!("{:+.3}", snap.sig.2));
                    row(ui, "α", format!("{:+.3}", k_ab * snap.alpha));
                    row(ui, "β", format!("{:+.3}", k_ab * snap.beta));
                    row(ui, "0", format!("{:+.3}", k_z * snap.zero));
                    row(ui, "d", format!("{:+.3}", k_ab * snap.d));
                    row(ui, "q", format!("{:+.3}", k_ab * snap.q));
                    row(ui, "|v|", format!("{:.3}", mag));
                    row(ui, "∠v", format!("{:+.1}°", ang));
                    row(ui, "θ frame", format!("{:.1}°", theta));
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let end_time = self.stop_time;
            let step_size = end_time / 1000.0;

            if !self.paused {
                let dt = ctx.input(|i| i.stable_dt) as f64;
                // Base speed: 5 seconds to draw full end_time
                let base_speed = end_time / 20.0;
                let actual_speed = base_speed * self.anim_duration;
                self.anim_time += dt * actual_speed;

                if self.anim_time > end_time {
                    self.anim_time = end_time;
                } else {
                    ctx.request_repaint();
                }
            }
            if self.anim_time > end_time {
                self.anim_time = end_time;
            }

            let delta_rad = self.delta.to_radians();
            let (k_ab, k_z) = self.scale_factors();

            let mut a_pts = vec![];
            let mut b_pts = vec![];
            let mut c_pts = vec![];
            let mut alpha_pts = vec![];
            let mut beta_pts = vec![];
            let mut zero_pts = vec![];
            let mut d_pts = vec![];
            let mut q_pts = vec![];
            let mut a_rev_pts = vec![];
            let mut b_rev_pts = vec![];
            let mut c_rev_pts = vec![];
            let mut ab_locus = vec![];
            let mut dq_locus = vec![];

            let mut t = 0.0;
            while t < self.anim_time {
                let (p_a, p_b, p_c) = self.phases_at(t, 0.0);
                a_pts.push([t, p_a]); b_pts.push([t, p_b]); c_pts.push([t, p_c]);

                let (alpha, beta, zero1) = transforms::abc_to_alpha_beta_0(p_a, p_b, p_c);
                alpha_pts.push([t, k_ab * alpha]);
                beta_pts.push([t, k_ab * beta]);
                zero_pts.push([t, k_z * zero1]);
                ab_locus.push([k_ab * alpha, k_ab * beta]);

                let wt_ref = self.park_wt(t);
                let (pd, pq, _) = transforms::alpha_beta_0_to_dq0(alpha, beta, zero1, wt_ref, delta_rad);
                d_pts.push([t, k_ab * pd]);
                q_pts.push([t, k_ab * pq]);
                dq_locus.push([k_ab * pd, k_ab * pq]);

                let (r_a, r_b, r_c) = transforms::dq0_to_abc(pd, pq, zero1, wt_ref, delta_rad);
                a_rev_pts.push([t, r_a]); b_rev_pts.push([t, r_b]); c_rev_pts.push([t, r_c]);

                t += step_size;
            }

            // Phasor state at the inspected instant (the clicked timestamp,
            // or the animation's leading edge when no cursor is set). Each
            // arrow tip is (quadrature, instantaneous), so its y coordinate is
            // exactly the value on the matching time plot at that instant.
            let t_now = self.cursor_time.map_or(self.anim_time, |c| c.min(self.anim_time));
            let marker = self.cursor_time.map(|c| c.min(self.anim_time));
            let mut clicked_time: Option<f64> = None;
            let snap = self.snapshot(t_now);
            let show_dq_axes = self.show_dq_axes;
            let show_construction = self.show_construction;

            // Fading tip trails over the last third of an electrical period.
            let trail_len = 0.35 / self.freq;
            let trail_n = 40;
            let mut trails: [Vec<[f64; 2]>; 3] = [vec![], vec![], vec![]];
            let mut trails_rev: [Vec<[f64; 2]>; 3] = [vec![], vec![], vec![]];
            for i in 0..=trail_n {
                let tt = t_now - trail_len * (i as f64) / (trail_n as f64);
                if tt < 0.0 {
                    break;
                }
                let s = self.snapshot(tt);
                trails[0].push([s.quad.0, s.sig.0]);
                trails[1].push([s.quad.1, s.sig.1]);
                trails[2].push([s.quad.2, s.sig.2]);
                trails_rev[0].push([s.rec_quad.0, s.rec_sig.0]);
                trails_rev[1].push([s.rec_quad.1, s.rec_sig.1]);
                trails_rev[2].push([s.rec_quad.2, s.rec_sig.2]);
            }
            let [trail_a, trail_b, trail_c] = trails;
            let [trail_ra, trail_rb, trail_rc] = trails_rev;

            let mult = if self.is_rms { SQRT_2 } else { 1.0 };
            let h_extra = 1.0 + (self.harm5 + self.harm7) / 100.0;
            let r_fund = (mult * self.mag_a.max(self.mag_b).max(self.mag_c)).max(1e-3);
            let r_abc = r_fund * h_extra;
            let r_ab = r_abc * k_ab;
            let y_abc = r_abc * 1.15;
            let y_ab = r_abc * k_ab.max(k_z) * 1.15;
            let y_dq = r_ab * 1.15;

            // Draws the shared time cursor and records clicks/drags on a time
            // plot so every graph can jump to the same inspected instant.
            let mut cursor_ui = |p: &mut PlotUi| {
                if let Some(tm) = marker {
                    p.vline(VLine::new(tm).color(COL_CURSOR).style(LineStyle::dashed_loose()));
                }
                let resp = p.response();
                if resp.clicked() || resp.dragged() {
                    if let Some(pos) = p.pointer_coordinate() {
                        clicked_time = Some(pos.x);
                    }
                }
            };

            // Color key, pinned above the scrolling rows so every trace, axis
            // and vector stays identifiable while browsing any plot.
            let swatch = |ui: &mut egui::Ui, color: Color32, text: &str| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, color);
                ui.label(text);
            };
            ui.horizontal_wrapped(|ui| {
                ui.label("Key:");
                swatch(ui, COL_A, "A");
                swatch(ui, COL_B, "B");
                swatch(ui, COL_C, "C");
                ui.separator();
                swatch(ui, COL_ALPHA, "α");
                swatch(ui, COL_BETA, "β");
                swatch(ui, COL_ZERO, "0 (zero-seq, dashed)");
                ui.separator();
                swatch(ui, COL_D, "d axis");
                swatch(ui, COL_Q, "q axis");
                ui.separator();
                swatch(ui, COL_VEC, "space vector");
                swatch(ui, Color32::from_gray(140), "locus");
                swatch(ui, COL_CURSOR, "time cursor (dashed)");
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let time_plot_width = (ui.available_width() - RADIAL_WIDTH - 24.0).max(200.0);

                ui.heading("ABC Waves");
                ui.horizontal(|ui| {
                    Plot::new("abc").width(time_plot_width).height(ROW_HEIGHT)
                        .include_x(0.0).include_x(end_time).include_y(-y_abc).include_y(y_abc)
                        .allow_drag(false)
                        .show(ui, |p| {
                            p.line(Line::new(PlotPoints::new(a_pts)).name("A").color(COL_A));
                            p.line(Line::new(PlotPoints::new(b_pts)).name("B").color(COL_B));
                            p.line(Line::new(PlotPoints::new(c_pts)).name("C").color(COL_C));
                            cursor_ui(p);
                        });
                    radial_plot("abc_radial", r_abc * 1.05).show(ui, |p| {
                        p.line(circle_line(r_fund));
                        p.line(Line::new(PlotPoints::new(trail_a)).color(COL_A.gamma_multiply(0.35)));
                        p.line(Line::new(PlotPoints::new(trail_b)).color(COL_B.gamma_multiply(0.35)));
                        p.line(Line::new(PlotPoints::new(trail_c)).color(COL_C.gamma_multiply(0.35)));
                        p.arrows(phasor([snap.quad.0, snap.sig.0], COL_A));
                        p.arrows(phasor([snap.quad.1, snap.sig.1], COL_B));
                        p.arrows(phasor([snap.quad.2, snap.sig.2], COL_C));
                    });
                });

                ui.heading("Alpha-Beta-0");
                ui.horizontal(|ui| {
                    Plot::new("ab").width(time_plot_width).height(ROW_HEIGHT)
                        .include_x(0.0).include_x(end_time).include_y(-y_ab).include_y(y_ab)
                        .allow_drag(false)
                        .show(ui, |p| {
                            p.line(Line::new(PlotPoints::new(alpha_pts)).name("Alpha").color(COL_ALPHA));
                            p.line(Line::new(PlotPoints::new(beta_pts)).name("Beta").color(COL_BETA));
                            p.line(
                                Line::new(PlotPoints::new(zero_pts))
                                    .name("Zero")
                                    .color(COL_ZERO)
                                    .width(2.5)
                                    .style(LineStyle::dashed_dense()),
                            );
                            cursor_ui(p);
                        });
                    radial_plot("ab_radial", r_ab * 1.05).show(ui, |p| {
                        p.line(circle_line(r_fund * k_ab));
                        if show_construction {
                            // Clarke as geometry: each phase value pulses along
                            // its fixed 120°-spaced axis; the head-to-tail sum
                            // of the three contributions IS the space vector.
                            let dirs = [
                                [1.0, 0.0],
                                [(2.0 * PI / 3.0).cos(), (2.0 * PI / 3.0).sin()],
                                [(4.0 * PI / 3.0).cos(), (4.0 * PI / 3.0).sin()],
                            ];
                            let vals = [snap.sig.0, snap.sig.1, snap.sig.2];
                            let cols = [COL_A, COL_B, COL_C];
                            let (mut sx, mut sy) = (0.0, 0.0);
                            for i in 0..3 {
                                p.line(
                                    axis_line(dirs[i], r_ab, cols[i].gamma_multiply(0.25))
                                        .style(LineStyle::dashed_loose()),
                                );
                                let vx = k_ab * (2.0 / 3.0) * vals[i] * dirs[i][0];
                                let vy = k_ab * (2.0 / 3.0) * vals[i] * dirs[i][1];
                                p.arrows(arrow([sx, sy], [sx + vx, sy + vy], cols[i]));
                                sx += vx;
                                sy += vy;
                            }
                        }
                        if show_dq_axes {
                            let ud = [snap.theta.sin(), -snap.theta.cos()];
                            let uq = [snap.theta.cos(), snap.theta.sin()];
                            p.line(axis_line(ud, r_ab, COL_D.gamma_multiply(0.5)));
                            p.line(axis_line(uq, r_ab, COL_Q.gamma_multiply(0.5)));
                            p.arrows(phasor([0.9 * r_ab * ud[0], 0.9 * r_ab * ud[1]], COL_D));
                            p.arrows(phasor([0.9 * r_ab * uq[0], 0.9 * r_ab * uq[1]], COL_Q));
                        }
                        p.line(Line::new(PlotPoints::new(ab_locus)).color(Color32::from_gray(140)));
                        p.arrows(phasor([k_ab * snap.alpha, k_ab * snap.beta], COL_VEC));
                    });
                });

                ui.heading("dq");
                ui.horizontal(|ui| {
                    Plot::new("dq").width(time_plot_width).height(ROW_HEIGHT)
                        .include_x(0.0).include_x(end_time).include_y(-y_dq).include_y(y_dq)
                        .allow_drag(false)
                        .show(ui, |p| {
                            p.line(Line::new(PlotPoints::new(d_pts)).name("d").color(COL_D));
                            p.line(Line::new(PlotPoints::new(q_pts)).name("q").color(COL_Q));
                            cursor_ui(p);
                        });
                    radial_plot("dq_radial", r_ab * 1.05).show(ui, |p| {
                        p.line(circle_line(r_fund * k_ab));
                        p.hline(HLine::new(0.0).color(COL_D.gamma_multiply(0.5)));
                        p.vline(VLine::new(0.0).color(COL_Q.gamma_multiply(0.5)));
                        p.line(Line::new(PlotPoints::new(dq_locus)).color(Color32::from_gray(140)));
                        p.arrows(phasor([k_ab * snap.d, k_ab * snap.q], COL_VEC));
                    });
                });

                ui.heading("Reconstructed ABC (Reverse Transform)");
                ui.horizontal(|ui| {
                    Plot::new("abc_rev").width(time_plot_width).height(ROW_HEIGHT)
                        .include_x(0.0).include_x(end_time).include_y(-y_abc).include_y(y_abc)
                        .allow_drag(false)
                        .show(ui, |p| {
                            p.line(Line::new(PlotPoints::new(a_rev_pts)).name("A_rev").color(COL_A));
                            p.line(Line::new(PlotPoints::new(b_rev_pts)).name("B_rev").color(COL_B));
                            p.line(Line::new(PlotPoints::new(c_rev_pts)).name("C_rev").color(COL_C));
                            cursor_ui(p);
                        });
                    radial_plot("abc_rev_radial", r_abc * 1.05).show(ui, |p| {
                        p.line(circle_line(r_fund));
                        p.line(Line::new(PlotPoints::new(trail_ra)).color(COL_A.gamma_multiply(0.35)));
                        p.line(Line::new(PlotPoints::new(trail_rb)).color(COL_B.gamma_multiply(0.35)));
                        p.line(Line::new(PlotPoints::new(trail_rc)).color(COL_C.gamma_multiply(0.35)));
                        p.arrows(phasor([snap.rec_quad.0, snap.rec_sig.0], COL_A));
                        p.arrows(phasor([snap.rec_quad.1, snap.rec_sig.1], COL_B));
                        p.arrows(phasor([snap.rec_quad.2, snap.rec_sig.2], COL_C));
                    });
                });
            });

            if let Some(x) = clicked_time {
                self.cursor_time = Some(x.clamp(0.0, end_time));
                ctx.request_repaint();
            }
        });
    }
}
