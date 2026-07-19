#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod transforms;

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::f64::consts::PI;

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
    is_rms: bool,
    fault: String,
    sag: f64,
    anim_time: f64,
    stop_time: f64,
    anim_duration: f64,
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
            freq: 60.0,
            delta: 0.0,
            is_rms: true,
            fault: "None".to_string(),
            sag: 50.0,
            anim_time: 0.0,
            stop_time: 0.1667,
            anim_duration: 1.0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("controls").show(ctx, |ui| {
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
            ui.checkbox(&mut self.is_rms, "RMS values");
            
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
            if ui.button("Restart Animation").clicked() {
                self.anim_time = 0.0;
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let end_time = self.stop_time;
            let step_size = end_time / 1000.0;

            let dt = ctx.input(|i| i.stable_dt) as f64;
            // Base speed: 5 seconds to draw full end_time
            let base_speed = end_time / 5.0; 
            let actual_speed = base_speed * self.anim_duration;
            self.anim_time += dt * actual_speed;
            
            if self.anim_time > end_time {
                self.anim_time = end_time; 
            } else {
                ctx.request_repaint(); 
            }

            let mut a_pts = vec![];
            let mut b_pts = vec![];
            let mut c_pts = vec![];
            let mut alpha_pts = vec![];
            let mut beta_pts = vec![];
            let mut d_pts = vec![];
            let mut q_pts = vec![];
            let mut a_rev_pts = vec![];
            let mut b_rev_pts = vec![];
            let mut c_rev_pts = vec![];

            let mut t = 0.0;
            while t < self.anim_time {
                let wt = 2.0 * PI * self.freq * t;
                let mult = if self.is_rms { std::f64::consts::SQRT_2 } else { 1.0 };
                let mut p_a = mult * self.mag_a * (wt + self.ang_a.to_radians()).sin();
                let mut p_b = mult * self.mag_b * (wt + self.ang_b.to_radians()).sin();
                let mut p_c = mult * self.mag_c * (wt + self.ang_c.to_radians()).sin();
                
                if t >= end_time / 4.0 && t <= 3.0 * end_time / 4.0 {
                    match self.fault.as_str() {
                        "Monophasic A" => p_a = 0.0,
                        "Monophasic B" => p_b = 0.0,
                        "Monophasic C" => p_c = 0.0,
                        "Two-phase A-B-ground" => { p_a = 0.0; p_b = 0.0; }
                        "Two-phase A-B" => p_a = p_b,
                        "Two-phase B-C-ground" => { p_b = 0.0; p_c = 0.0; }
                        "Two-phase B-C" => p_b = p_c,
                        "Two-phase C-A-ground" => { p_c = 0.0; p_a = 0.0; }
                        "Two-phase C-A" => p_c = p_a,
                        "Three-phase-ground" => { p_c = 0.0; p_b = 0.0; p_a = 0.0; }
                        "Three-phase" => { p_c = p_a; p_b = p_a; }
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
                
                a_pts.push([t, p_a]); b_pts.push([t, p_b]); c_pts.push([t, p_c]);

                let (alpha, beta, zero1) = transforms::abc_to_alpha_beta_0(p_a, p_b, p_c);
                alpha_pts.push([t, alpha]); beta_pts.push([t, beta]);

                let (pd, pq, _) = transforms::alpha_beta_0_to_dq0(alpha, beta, zero1, wt, self.delta.to_radians());
                d_pts.push([t, pd]); q_pts.push([t, pq]);
                
                // Use unused functions and build a reverse plot array
                let _ = transforms::alpha_beta_0_to_abc(alpha, beta, zero1);
                let _ = transforms::abc_to_dq0(p_a, p_b, p_c, wt, self.delta.to_radians());
                
                let (r_a, r_b, r_c) = transforms::dq0_to_abc(pd, pq, zero1, wt, self.delta.to_radians());
                a_rev_pts.push([t, r_a]); b_rev_pts.push([t, r_b]); c_rev_pts.push([t, r_c]);
                
                t += step_size;
            }

            ui.vertical(|ui| {
                ui.heading("ABC Waves");
                Plot::new("abc").height(150.0).include_x(0.0).include_x(end_time).include_y(-15.0).include_y(15.0).show(ui, |p| {
                    p.line(Line::new(PlotPoints::new(a_pts.clone())).name("A"));
                    p.line(Line::new(PlotPoints::new(b_pts.clone())).name("B"));
                    p.line(Line::new(PlotPoints::new(c_pts.clone())).name("C"));
                });
                ui.heading("Alpha-Beta");
                Plot::new("ab").height(150.0).include_x(0.0).include_x(end_time).include_y(-15.0).include_y(15.0).show(ui, |p| {
                    p.line(Line::new(PlotPoints::new(alpha_pts)).name("Alpha"));
                    p.line(Line::new(PlotPoints::new(beta_pts)).name("Beta"));
                });
                ui.heading("dq");
                Plot::new("dq").height(150.0).include_x(0.0).include_x(end_time).include_y(-15.0).include_y(15.0).show(ui, |p| {
                    p.line(Line::new(PlotPoints::new(d_pts)).name("d"));
                    p.line(Line::new(PlotPoints::new(q_pts)).name("q"));
                });
                ui.heading("Reconstructed ABC (Reverse Transform)");
                Plot::new("abc_rev").height(150.0).include_x(0.0).include_x(end_time).include_y(-15.0).include_y(15.0).show(ui, |p| {
                    p.line(Line::new(PlotPoints::new(a_rev_pts)).name("A_rev"));
                    p.line(Line::new(PlotPoints::new(b_rev_pts)).name("B_rev"));
                    p.line(Line::new(PlotPoints::new(c_rev_pts)).name("C_rev"));
                });
            });
        });
    }
}
