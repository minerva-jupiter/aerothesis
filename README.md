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

##### Formula for Ocillation

From above,
