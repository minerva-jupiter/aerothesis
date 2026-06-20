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

</details>

x(,f and v_f) formuler is 

$$x[n] = \frac{b_0 f[n] + b_1 f[n-1] + b_2 f[n-2] - a_1 x[n-1] - a_2 x[n-2]}{a_0}$$

$$f[n] = \pm \frac{1}{2} \rho v_f[n]^2 g[n]$$

$$v_f[n] = \frac{-\alpha + \sqrt{\alpha^2 + 4 B[n] \Gamma[n-1]}}{2 B[n]}$$

#### Resonance Part

<details>
<summary>Acoustic Simulation Logic: Displacement-Based Delay Line</summary>

Rather than simulating wave reflection through complex fluid dynamics (changes in density or tube stiffness), this model treats acoustic wave propagation as a delay-based system. We rely on the physical principle that acoustic energy dissipates more rapidly at higher frequencies, which we implement as a damping model applied to the displacement velocity.

#### 1. Damping Mechanism

Energy in an acoustic system is proportional to the square of the time derivative of displacement ($(\partial x / \partial t)^2$). We apply a damping constant $a$ to this derivative. This effectively attenuates higher-frequency components, as their energy dissipates faster than lower-frequency components. Given an input displacement $x_n$, a delayed resonant displacement $x_{\text{resonance}}$, and the total previous displacement $x_{\text{prev}}$, the system state is updated as:

$$x = a (x_{\text{prev}} - (x_n + x_{\text{resonance}}))^2$$

#### 2. Physical Validity (D’Alembert’s Solution)

The simplification of representing reflection as a pure time delay with a coefficient is mathematically rooted in the 1D wave equation:

$$\frac{\partial^2 p}{\partial t^2} - c^2 \frac{\partial^2 p}{\partial x^2} = 0$$

According to **D’Alembert’s solution**, any wave $p(x, t)$ can be decomposed into forward-traveling ($f$) and backward-traveling ($g$) waves:


$$p(x, t) = f(t - x/c) + g(t + x/c)$$

At the boundary $x=L$, we apply the following conditions:

* **Open End:** Pressure must be zero ($p=0$), leading to $g(t + L/c) = -f(t - L/c)$. The wave reflects with a phase inversion (coefficient $-1$).
* **Closed End:** Velocity must be zero ($\partial p/\partial x = 0$), leading to $g(t + L/c) = f(t - L/c)$. The wave reflects with its phase preserved (coefficient $+1$).

Consequently, calculating the resonance by multiplying the previously delayed displacement by a reflection coefficient is analytically equivalent to solving the wave equation for linear media.

#### 3. Defining the Delay Time

Given a note frequency $f$ and the speed of sound $c$, the wavelength $\lambda$ is defined as $\lambda = c/f$.

* **Open Pipe:** $\lambda = 2L \implies \text{round-trip time} = 2L/c = 1/f$.
* **Closed Pipe:** $\lambda = 4L \implies \text{round-trip time} = 4L/c = 2/f$.

Thus, the required delay samples can be derived directly from the frequency $f$ and sample rate $fs$ without needing explicit values for tube length $L$ or sound speed $c$.

*Note: While applying a low-pass filter to the output would achieve a similar spectral result, this implementation utilizes an explicit wave-propagation model to maintain physical rigor and simulate the dynamic behavior of the air column.*

</details>

The core simulation is based on a displacement-driven delay-line model, where the system state at time $n$ is determined by the input $x_n$ and the resonant wave $x_{\text{resonance}}$ returning from the pipe's boundary.

1. System Update Equation

The total displacement $x[n]$ is calculated as a damped non-linear function of the input and the delayed resonant state. Given a damping constant $a$ ($0 < a \le 1$):

$$x[n] = a \cdot \left( x[n-1] - (x_{\text{in}}[n] + x_{\text{resonance}}[n]) \right)^2$$

Where $x[n-1]$ represents the previous total displacement, capturing the system's memory.

2. Resonant Feedback (Delay and Reflection)

The resonant component $x_{\text{resonance}}$ is the delayed state derived from the pipe's boundary conditions. Given a delay buffer $D$ of length $T$ (where $T = f_s / f$), the resonance is defined by the reflection coefficient $R$:

$$x_{\text{resonance}}[n] = R \cdot \text{buffer}[n - T]$$

* **For Open Pipes (Open-Open):**
* Reflection occurs twice per round-trip with a phase inversion, resulting in $R = 1$ (net phase preserved).


* **For Closed Pipes (Closed-Open):**
* Reflection occurs once with phase inversion and once with phase preservation, resulting in $R = -1$ (net phase inversion per round-trip).



3. Signal Flow Summary

To maintain a stable simulation without algebraic loops, the signal flow follows this recursive update per sample:

1. **Retrieve:** $`x_{\text{res}} = R \cdot \text{delay\_buffer}[\text{ptr}]`$
2. **Compute:** $`x_{\text{curr}} = a \cdot (x_{\text{prev}} - (x_{\text{in}} + x_{\text{res}}))^2`$
3. **Update:** $`\text{delay\_buffer}[\text{ptr}] = x_{\text{curr}}`$
4. **Advance:** $`\text{ptr} = (\text{ptr} + 1) \pmod T`$

This approach effectively emulates the harmonic series and spectral decay of real instruments by utilizing the time-domain round-trip of the displacement wave as the primary oscillator, while the non-linear term $( \dots )^2$ provides the necessary harmonic distortion and energy dissipation.
