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

##### TL;DR Derivation of the simulation formula

###### 1. Formulation of Time-Varying Differential Equations and Trapezoidal Integrals

When mass changes over time, the equation of motion should ideally be described not as $m \frac{dv}{dt} = \dots$, but as the time derivative of momentum $p = m v$, $\frac{dp}{dt} = \dots$. However, in the physical modeling of reeds and lips, it is common to take the approximation that "at each instant, it behaves as a harmonic oscillator with the current mass, damping, and elasticity." Therefore, we start with the following system of differential equations.


$$\frac{dx(t)}{dt} = v(t)$$

$$\frac{dv(t)}{dt} = \frac{1}{m(t)} \left( F(t) - r(t) v(t) - k(t) x(t) \right)$$

Integrate both sides using the trapezoidal approximation (bilinear transformation) over the interval from time $(n-1)T$ to $nT$. For convenience, the values ​​at time $nT$ are denoted as $x[n], v[n], m[n], r[n], k[n], F[n]$.

Expression at position $x[n]$

$$x[n] - x[n-1] = \frac{T}{2} \left( v[n] + v[n-1] \right) \quad \cdots \text{(Equation 1)}$$

Expression of velocity $v[n]$

$$v[n] - v[n-1] = \frac{T}{2} \left( \frac{1}{m[n]}\big( F[n] - r[n]v[n] - k[n]x[n] \big) + \frac{1}{m[n-1]}\big( F[n-1] - r[n-1]v[n-1] - k[n-1]x[n-1] \big) \right) \quad \cdots \text{(Equation 2)}$$

---

###### 2. Resolving the Time-Varying Algebraic Loop

From the system of equations Equations 1 and 2, we eliminate the future velocity $v[n]$ and rearrange it into an explicit form of $x[n]$ that can be calculated using the current sample.

First, we isolate the current velocity $v[n]$ from Equation 1.

$$v[n] = \frac{2}{T}(x[n] - x[n-1]) - v[n-1] \quad \cdots \text{(Equation 3)}$$

We substitute this Equation 3 only for $v[n]$ on the right-hand side of Equation 2. This allows us to separate the terms containing unknown future variables to the left-hand side and the terms containing known past variables (states of $n-1$ and $n-2$) to the right-hand side.

Substituting Equation 3 and rearranging, we derive the following algebraic equation.

$$\left( \frac{4m[n]}{T^2} + \frac{2r[n]}{T} + k[n] \right) x[n] = \left( \frac{4m[n]}{T^2} + \frac{2r[n]}{T} \right) x[n-1] + 2m[n]v[n-1] + F[n] + \frac{m[n]}{m[n-1]} \left( F[n-1] - r[n-1]v[n-1] - k[n-1]x[n-1] \right)$$

Here, to eliminate the further past velocity state $v[n-1]$, we use the relationship from equation 1 one step back, namely $v[n-1] = \frac{2}{T}(x[n-1] - x[n-2]) - v[n-2]$ While it's possible to complete the transformation by substituting values, in the implementation of the audio DSP, a method is adopted to reduce the computational load by maintaining and updating both "past displacement $x[n-1]$" and "past velocity $v[n-1]$" as state variables.

--

###### 3. Simulation Equation to be Updated Every Sample

At the current sample time $n$, when $m[n], r[n], k[n]$ are determined by input from the aerophone, the time-varying difference equation to be calculated is as follows.

1. Calculation of Time-Varying Coefficients

For each sample, the coefficient $A[n]$ is calculated from the current physical parameters. Past coefficients and terms involving mass ratios are multiplied as they are.

$$A[n] = \frac{4m[n]}{T^2} + \frac{2r[n]}{T} + k[n]$$

2. Determination of the current displacement $x[n]$

$$x[n] = \frac{1}{A[n]} \left[ \left( \frac{4m[n]}{T^2} + \frac{2r[n]}{T} \right) x[n-1] + 2m[n]v[n-1] + F[n] + \frac{m[n]}{m[n-1]} \left( F[n-1] - r[n-1]v[n-1] - k[n-1]x[n-1] \right) \right]$$

3. Updating the velocity $v[n]$ for the next sample

Using the obtained $x[n]$, the current velocity can be obtained from equation 3. Calculate $v[n]$ and update the state variable.

$$v[n] = \frac{2}{T}(x[n] - x[n-1]) - v[n-1]$$

</details>

##### Formula for Oscillation

From above, formula for oscillation is
$$x[n] = \frac{1}{\frac{4m[n]}{T^2} + \frac{2r[n]}{T} + k[n]} \left[ \left( \frac{4m[n]}{T^2} + \frac{2r[n]}{T} \right) x[n-1] + 2m[n]v[n-1] + F[n] + \frac{m[n]}{m[n-1]} \left( F[n-1] - r[n-1]v[n-1] - k[n-1]x[n-1] \right) \right]$$
$$v[n] = \frac{2}{T}(x[n] - x[n-1]) - v[n-1]$$

Define bite strength $V_{\text{bite}}[n] \in [0.0, 1.0]$,and define breath strength $V_{\text{breath}}[n] \in [0.0, 1.0]$ mass $m[n]$ and others as follows:

$$m[n] = \text{base\_mass} \cdot \big( 1.0 - \text{bite\_mass\_scale} \cdot V_{\text{bite}}[n] \big)$$

$$r[n] = \text{base\_damping} \cdot \big( 1.0 + \text{bite\_damping\_scale} \cdot V_{\text{bite}}[n] \big) + \text{breath\_damping} \cdot V_{\text{breath}}[n]$$

$$k[n] = \text{base\_stiffness} \cdot \big( 1.0 + \text{bite\_stiffness\_scale} \cdot V_{\text{bite}}[n] \big)$$

$$F[n] = \text{pressure\_scale} \cdot V_{\text{breath}}[n] - \text{feedback\_gain} \cdot P_{\text{downstream}}[n]$$

P_downstream is under construction.

However, this model only produces damped oscillations.
By applying equations derived from fluid dynamics, such as those used by Reed, to F, we can create a continuous oscillation.

<details>
<summary>Physical Modeling: Fluid-Oscillator Interaction</summary>

The oscillator is driven by the fluid force $f(t)$ derived from the Bernoulli principle, incorporating the inertial effect of the fluid column.

###### 1. Fluid Dynamics

The fluid velocity $v_f(n)$ is governed by the momentum balance, where the air column with length $L$ acts as an inductor (inertia):

$$\frac{\rho L}{T} (v_f[n] - v_f[n-1]) + B[n] v_f[n]^2 = P(n)$$

Where:

* $\rho$: Air density.
* $L$: Effective length of the fluid column.
* $T$: Sampling interval.
* $B[n] = \frac{\rho}{4 (2 - x[n])^2}$: Geometry-dependent coefficient.
* $P(n)$: Effective pressure (Breath pressure minus feedback).

Solving for $v_f[n]$ at each sample provides the driving force:

$$v_f[n] = \frac{-A + \sqrt{A^2 + 4 B[n] (A v_f[n-1] + C[n-1])}}{2 B[n]}$$


*(where $A = \frac{\rho L}{T}$ and $C[n-1] = P - B[n-1] v_f[n-1]^2$)*

###### 2. Oscillator Coupling

The oscillator is defined by a second-order differential equation $m \ddot{x} + r \dot{x} + k x = f$. The external force $f[n]$ is mapped based on the selected instrument mode:

* **Saxophone Mode:**

$$f[n] = \frac{1}{2} \rho v_f[n]^2 (2 - x[n])$$



*Driven by negative pressure (suction) as air flows through the gap.*
* **Trumpet Mode:**

$$f[n] = -\frac{1}{2} \rho v_f[n]^2 (2 - x[n])$$



*Driven by positive pressure pushing the lips outward.*

###### 3. Discrete Implementation (Bilinear Transform)

The equation is discretized using the bilinear transform into a second-order difference equation:

$$a_0 x[n] + a_1 x[n-1] + a_2 x[n-2] = b_0 f[n] + b_1 f[n-1] + b_2 f[n-2]$$

The system preserves energy by accounting for parameter modulation (bite-dependent $m, k, r$) via energy-preserving compensation terms in the discrete state-space formulation.

</details>
