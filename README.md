# Aerothesis

## Building

After installing [Rust](https://rustup.rs/), you can compile Aerothesis as follows:

```shell
cargo xtask bundle aerothesis --release
```

## Design

Purpose of this repository is creating an expressive wind synthesizer, like real trumpets, saxophones and other instruments.

### Architecture

#### Temporary oscillation

This parts play a role of generating sounds like the reed on a saxophone or the lips on a trumpet.

<details>
<summary>TL;DR Derivation of the simulation formula</summary>

## Derivation of the simulation formula

### 1. Continuous-Time Physical Equations

#### Mechanical Oscillator (Reed Dynamics)

The mechanical movement of the reed is modeled as a damped mass-spring system driven by an external fluid force $f(t)$:

$$m \frac{d^2 x(t)}{dt^2} + r \frac{dx(t)}{dt} + k x(t) = f(t)$$

Where:

* $m$: Effective mass of the reed.
* $r$: Mechanical damping.
* $k$: Stiffness (restoring force coefficient).
* $x(t)$: Reed displacement ($x=0$ at rest, $x=2.0$ represents complete channel closure).

#### Fluid Dynamics (Pressure-Velocity Relation)

The airflow through the reed gap incorporates both the pressure drop across the orifice (Bernoulli's principle) and the acoustic/fluid inertia of the air mass within the channel:

$$\rho L \frac{dv_f(t)}{dt} + \frac{\rho}{4 g(t)^2} v_f(t)^2 = P(t)$$

Where:

* $\rho$: Air density ($1.2 \text{ kg/m}^3$).
* $L$: Effective length of the fluid column.
* $v_f(t)$: Fluid flow velocity.
* $P(t)$: Driving breath pressure.
* $g(t) = \max(2.0 - x(t), \epsilon)$: Dynamic aperture (gap width, clamped with a tiny $\epsilon$ to prevent division by zero).

#### Aeroelastic Coupling (Fluid Force)

The aerodynamic force $f(t)$ acting on the surface of the reed depends on the dynamic pressure and the geometry of the channel:

$$f(t) = \pm \frac{1}{2} \rho v_f(t)^2 g(t)$$

* **$\boldsymbol{+}$ (Positive Sign):** `SingleReed` Mode (Saxophone/Clarinet). The high velocity creates suction (Bernoulli effect) that pulls the reed toward closure.
* **$\boldsymbol{-}$ (Negative Sign):** `LipReed` Mode (Trumpet/Brass). The pressure pushes the lips outward to open the channel.

---

### 2. Discretization via Bilinear Transform

To compute this system inside a digital signal processor at sampling interval $T = 1 / f_s$, we map the continuous differential equations to discrete difference equations using the **Bilinear Transform (Tustin's Method)**.

The continuous derivative operator in the Laplace domain, $s$, is substituted by the discrete variable $z$ using the trapezoidal integration approximation:

$$s \approx \frac{2}{T} \frac{1 - z^{-1}}{1 + z^{-1}}$$

Applying this to the second-order derivative ($s^2$) yields:

$$s^2 \approx \frac{4}{T^2} \frac{1 - 2z^{-1} + z^{-2}}{1 + 2z^{-1} + z^{-2}}$$

#### Derivation of the Discrete Difference Equation

Substituting these into the mechanical transfer function $H(s) = \frac{X(s)}{F(s)} = \frac{1}{ms^2 + rs + k}$ gives:

$$\frac{X(z)}{F(z)} = \frac{1}{m \left(\frac{4}{T^2} \frac{1 - 2z^{-1} + z^{-2}}{1 + 2z^{-1} + z^{-2}}\right) + r \left(\frac{2}{T} \frac{1 - z^{-1}}{1 + z^{-1}}\right) + k}$$

Multiplying both the numerator and denominator by $(1 + 2z^{-1} + z^{-2})$ and grouping identical powers of $z^{-1}$, we clear the fraction fractions. To eliminate $1/T^2$ fractions and maximize numerical precision in single-precision floating-point math (`f32`), we multiply the entire equation by $T^2$:

$$\frac{X(z)}{F(z)} = \frac{T^2 (1 + 2z^{-1} + z^{-2})}{(4m + 2rT + kT^2) + (-8m + 2kT^2)z^{-1} + (4m - 2rT + kT^2)z^{-2}}$$

Thus, we obtain the standard **Direct Form I** difference equation coefficients:

* $b_0 = T^2$
* $b_1 = 2T^2$
* $b_2 = T^2$
* $a_0 = 4m + 2rT + kT^2$
* $a_1 = -8m + 2kT^2$
* $a_2 = 4m - 2rT + kT^2$

The dynamic calculation of the exact displacement $x[n]$ at the current time-step is explicitly resolved as:

$$x[n] = \frac{b_0 f[n] + b_1 f[n-1] + b_2 f[n-2] - a_1 x[n-1] - a_2 x[n-2]}{a_0}$$

---

### 3. Proof of Approximation Validity and Stability

We prove that this discrete equation is a highly appropriate mathematical approximation of the continuous physical system $x(t)$ based on three criteria.

#### Proof A: Frequency Mapping Consistency

The Bilinear Transform maps the entire continuous imaginary axis ($s = j\Omega$) onto the discrete unit circle ($z = e^{j\omega T}$). The mapping relationship is exactly:

$$\Omega = \frac{2}{T} \tan\left(\frac{\omega T}{2}\right)$$

For audio rates where the natural resonant frequency of the reed $\Omega_0 = \sqrt{k/m}$ satisfies $\Omega_0 \ll \frac{2}{T}$ (highly true since reed resonances are typically below $5\text{ kHz}$ and $T^{-1} = 44.1\text{ kHz}$), the Taylor expansion of the tangent function yields:

$$\Omega \approx \frac{2}{T} \left( \frac{\omega T}{2} \right) = \omega$$

This proves that in the audible band, the discrete frequency spectrum matches the physical continuous resonance behavior without severe warping.

#### Proof B: Unconditional Numerical Stability (Passivity Preservation)

A physical reed is a passive system that absorbs and dissipates energy via $r$. For a system to be stable in the discrete domain, its poles must lie strictly inside the unit circle ($|z| < 1$).

The continuous system poles lie in the Left-Half of the s-plane ($\text{Re}(s) < 0$) because $m, r, k > 0$. Under the bilinear mapping:

$$z = \frac{1 + \frac{T}{2}s}{1 - \frac{T}{2}s}$$

Taking the magnitude squared when $\text{Re}(s) = \sigma < 0$:

$$|z|^2 = \frac{(1 + \frac{T}{2}\sigma)^2 + (\frac{T}{2}\Omega)^2}{(1 - \frac{T}{2}\sigma)^2 + (\frac{T}{2}\Omega)^2}$$

Since $\sigma < 0$, it is mathematically guaranteed that $(1 + \frac{T}{2}\sigma)^2 < (1 - \frac{T}{2}\sigma)^2$, which proves $|z| < 1$.

> **Conclusion:** The algorithm is **unconditionally stable** regardless of sample rate modifications or sudden physical parameter adjustments ($m, k, r$ modulation via bite intensity), eliminating numerical explosion risks inherent in explicit forward-Euler methods.

#### Proof C: Order of Accuracy (Trapezoidal Match)

The bilinear transform is mathematically isomorphic to the trapezoidal integration rule. The local truncation error ($LTE$) of a trapezoidal approximation for a state vector $\mathbf{X}$ is bounded by:

$$LTE = \mathcal{O}(T^3)$$

Accumulated over a global simulation window, the overall approximation error scales as $\mathcal{O}(T^2)$ (Second-order accurate). Compared to standard first-order Euler methods ($\mathcal{O}(T)$), this guarantees that high-frequency physical transcripts (such as rapid transients during attacks or lip-buzzing regimes) are preserved with minimal numerical artificial damping.

</details>
