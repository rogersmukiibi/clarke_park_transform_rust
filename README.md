# Clarke & Park Transform Visualizer

An interactive desktop app for building intuition about the **Clarke (αβ0)** and **Park (dq0)** transforms, written in Rust with [egui](https://github.com/emilk/egui)/eframe.

Three-phase signals are animated in real time through the full transform chain — `abc → αβ0 → dq0 → abc` — with a synchronized phasor (radial) view next to every plot, so you can see both the waveforms *and* the rotating vectors they represent at any instant.

## Features

- **Four synchronized views**: ABC waveforms, Clarke (α, β and zero-sequence), Park (d, q), and the reconstructed ABC from the reverse transform.
- **Radial phasor views** beside each plot:
  - rotating per-phase phasors with fading tip trails (ABC and reconstruction),
  - the space vector with its locus, the rotating d/q axes, and the head-to-tail *construction* of the space vector from the three phase contributions (αβ view),
  - the same vector seen from the rotating frame (dq view).
- **Time cursor**: click or drag on any time plot and every graph — including all radial views and the numeric readout — jumps to that instant. Use *Follow Animation* to return to live tracking.
- **Numeric readout**: a, b, c, α, β, 0, d, q, space-vector magnitude/angle, and the frame angle θ at the inspected instant.
- **Park reference frame controls**: frame offset angle δ, and a frame frequency de-tune Δf that demonstrates slip (why real systems need a PLL).
- **Fault simulation** applied during the middle half of the timeline: single-phase, phase-to-phase (grounded and ungrounded), three-phase, and voltage sags. Ungrounded faults collapse the shorted phases to their common mean potential.
- **Harmonic injection**: 5th and 7th harmonics, which appear as the classic 6th-harmonic ripple in the dq frame.
- **Convention toggle**: amplitude-invariant (2/3) or power-invariant (√(2/3)) scaling.
- **Animation controls**: speed, pause/resume, restart.

## Running

```bash
cargo run
```

### WSL2 / WSLg note

On WSL2, winit's Wayland backend fails against WSLg's compositor (EGL cannot create a hardware surface and the app exits with `WinitEventLoop(ExitFailure(1))`). Force the X11 backend instead:

```bash
WAYLAND_DISPLAY= cargo run
```

To make this permanent, add to `~/.bashrc`:

```bash
if grep -qi microsoft /proc/version; then
    unset WAYLAND_DISPLAY
fi
```

## The math

The implementation (in [`src/transforms.rs`](src/transforms.rs)) uses the **amplitude-invariant** Clarke transform and a **sine-aligned** Park convention. Signals are generated as $x = m\sin(\omega t + \varphi)$.

**Clarke (abc → αβ0):**

```math
\alpha = \tfrac{2}{3}\left(a - \tfrac{b}{2} - \tfrac{c}{2}\right),\qquad
\beta  = \tfrac{2}{3}\cdot\tfrac{\sqrt{3}}{2}\,(b - c),\qquad
0 = \tfrac{1}{3}(a + b + c)
```

**Park (αβ → dq), with frame angle** $\theta = \omega_{ref}\,t + \delta$:

```math
d = \alpha\sin\theta - \beta\cos\theta,\qquad
q = \alpha\cos\theta + \beta\sin\theta
```

With a balanced positive-sequence set at nominal frequency and $\delta = 0$, this yields $d = m$ (the peak amplitude, or $\sqrt{2}\cdot m_{RMS}$ with RMS input) and $q = 0$. The zero-sequence component bypasses the rotation unchanged, which is why the reverse transform reproduces the original signals exactly — even for unbalanced faults, where the zero channel carries what the αβ plane cannot represent.

The inverse transforms are the exact matrix inverses, so `abc → αβ0 → dq0 → abc` is an identity for *any* input (including during faults and with a de-tuned reference frame).

When **power-invariant** display is selected, the αβ0/dq quantities are shown scaled by $\sqrt{3/2}$ (and the zero component by $\sqrt{3}$), matching the orthonormal $\sqrt{2/3}$ transform convention.

## Things to try

1. **Park in action** — drag *Frame δ*: d and q trade places while the ABC signals never change; the αβ radial shows the d-axis rotating away from the space vector.
2. **Slip** — set *Frame Δf* to a few Hz: d/q become slow sinusoids at the slip frequency and the dq vector starts rotating.
3. **Zero sequence** — select the *Monophasic A* fault: the dashed zero trace lights up during the fault while the space vector barely reacts; the reconstruction still matches perfectly.
4. **Negative sequence** — select an ungrounded two-phase fault and pause mid-fault: the αβ locus becomes an ellipse and d/q ripple at twice the fundamental.
5. **Harmonics** — add 5th/7th harmonic: the phasor tips trace epicyclic loops and dq shows 6th-harmonic ripple.
6. **Clarke geometrically** — watch the αβ construction chain: three pulsating vectors on fixed 120°-spaced axes summing head-to-tail onto the rotating space-vector tip.

## Project layout

| Path | Contents |
|---|---|
| [`src/main.rs`](src/main.rs) | UI, animation, fault model, phasor/radial views |
| [`src/transforms.rs`](src/transforms.rs) | Pure transform math (Clarke, Park, and inverses) |

## Dependencies

- [eframe](https://crates.io/crates/eframe) 0.29 — window/app framework
- [egui_plot](https://crates.io/crates/egui_plot) 0.29 — plotting
