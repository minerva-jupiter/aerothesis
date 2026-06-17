# Aerothesis

## Building

After installing [Rust](https://rustup.rs/), you can compile Aerothesis as follows:

```shell
cargo xtask bundle aerothesis --release
```

## Design

Purpose of this repository is creating an expressive wind synthesizer, like real trumpets, saxophones and other instruments.

### Architecture

#### Primary oscillation

This parts play a role of generating sounds like the reed on a saxophone or the lips on a trumpet.

<details>
<summary>TL;DR Derivation of the simulation formula</summary>

```markdown
## Physical Modeling and Discretization Process

This plugin simulates the sound generation mechanism of a reed instrument (or lip-reed instrument) by coupling a continuous-time mechanical oscillator with a non-linear fluid dynamics engine.

---

### 1. Fluid Dynamics Discretization & Velocity Derivation

#### Continuous-Time Fluid Equation

The pressure drop $P(t)$ across the orifice incorporates both the acoustic/fluid inertia of the air mass within the channel and Bernoulli's principle:

$$P(t) = \rho L \frac{dv_f(t)}{dt} + B(t) v_f(t)^2$$

Where:

* $\rho$: Air density ($1.2 \text{ kg/m}^3$).
* $L$: Effective length of the fluid column.
* $v_f(t)$: Fluid flow velocity.
* $B(t) = \frac{\rho}{4 g(t)^2}$: Geometry-dependent flow resistance coefficient ($g(t)$ is the dynamic aperture).

#### Bilinear Transform (Trapezoidal Integration)

To discretize the derivative term, we apply the bilinear transform, which is mathematically equivalent to the trapezoidal rule. The derivative of fluid velocity at step $n$ is approximated as:

$$\frac{dv_f(t)}{dt} \approx \frac{2}{T} (v_f[n] - v_f[n-1]) - \left.\frac{dv_f(t)}{dt}\right|_{n-1}$$

Substituting the continuous fluid equation at step $n-1$ into the historic derivative term yields:

$$\frac{dv_f(t)}{dt} \approx \frac{2}{T} (v_f[n] - v_f[n-1]) - \frac{1}{\rho L} \left( P[n-1] - B[n-1] v_f[n-1]^2 \right)$$

Substituting this approximation back into the continuous-time equation at step $n$:

$$P[n] = \rho L \left[ \frac{2}{T} (v_f[n] - v_f[n-1]) - \frac{1}{\rho L} \left( P[n-1] - B[n-1] v_f[n-1]^2 \right) \right] + B[n] v_f[n]^2$$

Expanding and organizing the equation into a quadratic form with respect to the current velocity $v_f[n]$:

$$B[n] v_f[n]^2 + \left( \frac{2\rho L}{T} \right) v_f[n] - \left[ P[n] + P[n-1] + \frac{2\rho L}{T} v_f[n-1] - B[n-1] v_f[n-1]^2 \right] = 0$$

#### Analytical Solution for Discrete Fluid Velocity

To solve for the physically valid (positive) root of this quadratic equation, let:

* $A = \frac{2\rho L}{T}$
* $C[n-1] = P[n] + P[n-1] + A v_f[n-1] - B[n-1] v_f[n-1]^2$

Applying the quadratic formula explicitly determines the discrete fluid velocity $v_f[n]$ at the current time-step:

$$v_f[n] = \frac{-A + \sqrt{A^2 + 4 B[n] C[n-1]}}{2 B[n]}$$

#### Mapping to Fluid Force $f[n]$

The calculated velocity $v_f[n]$ is immediately mapped to the aerodynamic force $f[n]$ acting on the reed surface based on the selected instrument mode:

$$f[n] = \pm \frac{1}{2} \rho v_f[n]^2 g[n]$$

* **$\boldsymbol{+}$ (Positive Sign):** `SingleReed` Mode (Saxophone). The high velocity creates suction (Bernoulli effect) that pulls the reed toward closure.
* **$\boldsymbol{-}$ (Negative Sign):** `LipReed` Mode (Trumpet). The pressure pushes the lips outward to open the channel.

---

### 2. Mechanical Oscillator Discretization

#### Continuous-Time Mechanical Equation

The movement of the mechanical reed is modeled as a damped mass-spring system driven by the derived fluid force $f(t)$:

$$m \frac{d^2 x(t)}{dt^2} + r \frac{dx(t)}{dt} + k x(t) = f(t)$$

Where $m$ is the effective mass, $r$ is the mechanical damping, $k$ is the stiffness, and $x(t)$ is the displacement.

#### Bilinear Transform of the Oscillator

We map the continuous differential system to the discrete $z$-domain by substituting the complex frequency $s$ via Tustin's method:

$$s \approx \frac{2}{T} \frac{1 - z^{-1}}{1 + z^{-1}}$$

Applying this substitution to the second-order mechanical transfer function $H(s) = \frac{X(s)}{F(s)} = \frac{1}{ms^2 + rs + k}$ yields:

$$\frac{X(z)}{F(z)} = \frac{1}{m \left(\frac{4}{T^2} \frac{1 - 2z^{-1} + z^{-2}}{1 + 2z^{-1} + z^{-2}}\right) + r \left(\frac{2}{T} \frac{1 - z^{-1}}{1 + z^{-1}}\right) + k}$$

Multiplying both the numerator and denominator by $(1 + 2z^{-1} + z^{-2})$ and scaling the entire equation by $T^2$ to eliminate fractional sampling intervals ensures maximum numerical precision in single-precision floating-point math (`f32`):

$$\frac{X(z)}{F(z)} = \frac{T^2 (1 + 2z^{-1} + z^{-2})}{(4m + 2rT + kT^2) + (-8m + 2kT^2)z^{-1} + (4m - 2rT + kT^2)z^{-2}}$$

This defines the standard **Direct Form I** difference equation coefficients:

* $b_0 = T^2, \quad b_1 = 2T^2, \quad b_2 = T^2$
* $a_0 = 4m + 2rT + kT^2$
* $a_1 = -8m + 2kT^2$
* $a_2 = 4m - 2rT + kT^2$

The exact discrete displacement $x[n]$ at the current time-step is calculated as:

$$x[n] = \frac{b_0 f[n] + b_1 f[n-1] + b_2 f[n-2] - a_1 x[n-1] - a_2 x[n-2]}{a_0}$$

---

### 3. Proof of Approximation Validity and Stability

#### Proof A: Frequency Mapping Consistency

The Bilinear Transform maps the continuous imaginary axis ($s = j\Omega$) onto the discrete unit circle ($z = e^{j\omega T}$) via the exact relationship:

$$\Omega = \frac{2}{T} \tan\left(\frac{\omega T}{2}\right)$$

For audio rates where the natural resonant frequency of the reed $\Omega_0 = \sqrt{k/m}$ satisfies $\Omega_0 \ll \frac{2}{T}$ (highly true since reed resonances are typically below $5\text{ kHz}$ and $T^{-1} = 44.1\text{ kHz}$), the Taylor expansion of the tangent function yields $\Omega \approx \omega$. This proves that the discrete resonance matches the continuous physical spectrum without severe high-frequency warping in the audible band.

#### Proof B: Unconditional Numerical Stability

A physical reed system is passive and absorbs/dissipates energy via $r$. The continuous system poles lie in the Left-Half of the s-plane ($\text{Re}(s) = \sigma < 0$) because $m, r, k > 0$. Under the bilinear mapping:

$$|z|^2 = \left| \frac{1 + \frac{T}{2}s}{1 - \frac{T}{2}s} \right|^2 = \frac{(1 + \frac{T}{2}\sigma)^2 + (\frac{T}{2}\Omega)^2}{(1 - \frac{T}{2}\sigma)^2 + (\frac{T}{2}\Omega)^2}$$

Since $\sigma < 0$, $(1 + \frac{T}{2}\sigma)^2 < (1 - \frac{T}{2}\sigma)^2$, mathematically guaranteeing $|z| < 1$.

> **Conclusion:** The system remains **unconditionally stable** regardless of real-time sampling rate modifications or aggressive parameter modulation ($m, k, r$ adjustments via bite intensity), eliminating numerical explosion risks common in forward-Euler methods.
```

</details>

x(,f and v_f) formuler is 

$$x[n] = \frac{b_0 f[n] + b_1 f[n-1] + b_2 f[n-2] - a_1 x[n-1] - a_2 x[n-2]}{a_0}$$

$$f[n] = \pm \frac{1}{2} \rho v_f[n]^2 g[n]$$

$$v_f[n] = \frac{-\alpha + \sqrt{\alpha^2 + 4 B[n] \Gamma[n-1]}}{2 B[n]}$$

#### Resonance Part

<details>
<summary>Physical Modeling of Boundary Dissipation in Open-Ended Acoustic Tubes</summary>

This document provides the theoretical background, mathematical derivation, and discrete-time modeling of the acoustic reflection and energy dissipation at the open end (bell) of a resonant tube. Rather than using an empirical low-pass filter to attenuate high-frequency components, this model derives the high-frequency decay directly from the **physical mechanics of particle motion and nonlinear aerodynamic resistance** at the boundary.

---

## 1. Physical Phenomenon: Open-End Reflection and Turbulence

When an acoustic wave propagating inside a tube reaches an open end, it transitions from a highly constrained, one-dimensional channel to a free, unconstrained three-dimensional space.

1. **Inertial Overshoot:** As a high-pressure (condensation) wave arrives at the exit, the air particles are suddenly liberated and accelerate outward into the ambient atmosphere. Due to their mass (inertia), these particles overshoot, evacuation the region just inside the exit. This creates a local localized low-pressure (rarefaction) zone, which propagates back into the tube as a phase-inverted ($-1$) reflection.
2. **Nonlinear Energy Dissipation ($v|v|$ Loss):** When air particles exhaust violently out of the tube orifice, the sharp edge causes flow separation, generating **local vortices and turbulence**. In fluid dynamics, this rapid change in cross-sectional area converts kinetic energy into unrecoverable thermal and acoustic radiation losses. The pressure drop $\Delta P$ across such an orifice is dominated by dynamic pressure, meaning it is proportional to the **square of the particle velocity** ($v^2$).

---

## 2. Continuous-Time Governing Equation

We model the air plug at the open end as an effective acoustic mass (inertial slug) $M_e$ subject to a nonlinear aerodynamic resistance proportional to $v|v|$ to ensure that the damping force always opposes the direction of flow.

Let $p_{total}(t)$ be the total acoustic pressure at the boundary, $S$ be the cross-sectional area of the tube, and $v(t)$ be the acoustic particle velocity. The force balance equation at the boundary $x = L$ is formulated as follows:

$$S \cdot p_{total}(t) = M_e \frac{dv(t)}{dt} + r_{loss} v(t) |v(t)|$$

Where:

* $M_e = \rho_0 S \Delta L$ is the effective mass of the air plug (where $\Delta L \approx 0.613 \cdot \text{radius}$ is the open-end boundary correction length).
* $r_{loss} = \frac{1}{2} \rho_0 S$ is the nonlinear loss coefficient derived from the dynamic pressure dissipation rate.
* $\rho_0$ is the ambient air density.

---

## 3. Boundary Coupling with Digital Waveguide

In a digital waveguide framework, the total pressure $p_{total}$ at the boundary is the sum of the incoming progressive wave (right-going wave $p^+$) and the reflected wave (left-going wave $p^-$):

$$p_{total}(t) = p^+(t) + p^-(t)$$

The particle velocity $v(t)$ is related to these traveling components via the characteristic acoustic impedance of the tube, $Z_0 = \frac{\rho_0 c}{S}$:

$$v(t) = \frac{1}{Z_0} \left( p^+(t) - p^-(t) \right)$$

By rewriting the reflected wave $p^-(t)$ in terms of the current velocity and the incident wave, we eliminate the algebraic loop:

$$p^-(t) = p^+(t) - Z_0 v(t)$$

Substituting this back into the total pressure equation yields:

$$p_{total}(t) = 2p^+(t) - Z_0 v(t)$$

---

## 4. Discrete-Time Derivation via Backward Euler Discretization

To solve the differential equation numerically without implicit algebraic loops, we apply a first-order Backward Euler finite difference approximation to the derivative term:

$$\frac{dv(t)}{dt} \approx \frac{v[n] - v[n-1]}{T}$$

Where $T = 1/f_s$ is the sampling period. Substituting the discrete derivative and the impedance-coupled total pressure into the continuous governing equation gives:

$$S \left( 2p^+[n] - Z_0 v[n] \right) = M_e \left( \frac{v[n] - v[n-1]}{T} \right) + r_{loss} v[n] |v[n]|$$

Assuming a positive outward flow ($v[n] > 0 \implies v[n]|v[n]| = v[n]^2$) during the primary exhaust cycle, we collect terms to form a standard quadratic equation with respect to the current velocity $v[n]$:

$$r_{loss} v[n]^2 + \left( \frac{M_e}{T} + S Z_0 \right) v[n] - \left( 2S p^+[n] + \frac{M_e}{T} v[n-1] \right) = 0$$

Let us define the discrete time-invariant and time-varying coefficients as:

$$\begin{aligned}
A_{bc} &= r_{loss} \\
B_{bc} &= \frac{M_e}{T} + S Z_0 \\
C_{bc}[n] &= 2S p^+[n] + \frac{M_e}{T} v[n-1]
\end{aligned}$$

The equation reduces to:

$$A_{bc} v[n]^2 + B_{bc} v[n] - C_{bc}[n] = 0$$

Applying the quadratic formula yields the explicit, stable algebraic solution for the particle velocity at the current sample step $n$:

$$v[n] = \frac{-B_{bc} + \sqrt{B_{bc}^2 + 4 A_{bc} C_{bc}[n]}}{2 A_{bc}}$$

Once $v[n]$ is evaluated, the true physical reflected wave $p^-[n]$ injected back into the delay line is determined deterministically:

$$p^-[n] = p^+[n] - Z_0 v[n]$$

---

## 5. Why This Causes High-Frequency Decay

This mathematical formulation inherently explains why higher frequencies experience more severe damping without artificially implementing a digital filter:

1. **Velocity-Dependent Damping:** High-frequency components undergo rapid structural changes over time ($\frac{dv}{dt}$ is large), forcing higher transient particle velocities $v[n]$.
2. **Quadratic Penalty:** Because the dissipative term scales with $v[n]^2$, these high-velocity, high-frequency transients encounter a nonlinearly magnified resistance compared to slow, low-frequency pressure oscillations.
3. **Impedance Matching Shift:** At low velocities, the quadratic term vanishes, and the system matches an ideal open boundary, achieving an inversion coefficient close to $-1$. At high velocities, the effective boundary impedance changes due to $r_{loss}v^2$, allowing energy to escape into the environment as radiation or heat rather than reflecting back into the pipe.

</details>
