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

###### 1. Formulation of the Equations of Motion in Continuous Time

Let the displacement of the reed or lip be $x(t)$ and the velocity be $v(t) = \dot{x}(t)$. Given the mass $m$, the braking coefficient $r$, the elastic modulus $k$, and the driving force $F(t) = S \cdot \Delta P(t)$, the system can be described as the following system of first-order differential equations.


$$\frac{dx(t)}{dt} = v(t)$$

$$m \frac{dv(t)}{dt} + r v(t) + k x(t) = F(t) \implies \frac{dv(t)}{dt} = \frac{1}{m} \left( F(t) - r v(t) - k x(t) \right)$$

---

###### 2. Application of Bilinear Transform (Trapezoidal Integralis)

Let the sampling period be $T$, and consider the integral from continuous time to the digital time axis $t = nT$.

The essence of the bilinear transform is to approximate the integrand using a trapezoidal approximation (multiplying the mean value at both ends by the interval width $T$) for the integral from time $(n-1)T$ to $nT$.

####### Discretization of position $x[n]$

$$\int_{(n-1)T}^{nT} \frac{dx(t)}{dt} dt = \int_{(n-1)T}^{nT} v(t) dt$$

$$x[n] - x[n-1] = \frac{T}{2} \left( v[n] + v[n-1] \right) \quad \cdots \text{(Equation 1)}$$

####### Discretization of velocity $v[n]$

$$\int_{(n-1)T}^{nT} \frac{dv(t)}{dt} dt = \int_{(n-1)T}^{nT} \frac{1}{m} \left( F(t) - r v(t) - k x(t) \right) dt$$

$$v[n] - v[n-1] = \frac{T}{2m} \left( F[n] + F[n-1] - r(v[n] + v[n-1]) - k(x[n] + x[n-1]) \right) \quad \cdots \text{(Equation 2)}$$

---

###### 3. Resolving the Algebraic Loop and Deriving the Simulation Formula

Equations 1 and 2 are implicit relations that include unknown future values ​​(values ​​at time $n$) on both the left and right sides, forming a so-called algebraic loop. We transform this into an explicit update formula that can be computed using the current sample.

First, we solve Equation 1 for $v[n]$.


$$v[n] = \frac{2}{T}(x[n] - x[n-1]) - v[n-1] \quad \cdots \text{(Equation 3)}$$

Substituting Equation 3 into Equation 2 and rearranging for the future displacement $x[n]$, we multiply both sides of Equation 2 by $2m$ and rearrange. The final difference equation is:

$$\left( \frac{4m}{T^2} + \frac{2r}{T} + k \right) x[n] = \left( \frac{8m}{T^2} - k \right) x[n-1] - \left( \frac{4m}{T^2} - \frac{2r}{T} \right) x[n-2] + F[n] + F[n-1]$$

Here, the coefficients consisting of time-invariant physical constants are defined as follows:

$$A = \frac{4m}{T^2} + \frac{2r}{T} + k, \quad B = \frac{8m}{T^2} - k, \quad C = \frac{4m}{T^2} - \frac{2r}{T}$$

This determines the approximately correct simulation equation that should be evaluated for each sample within loops such as nih-plug.


$$x[n] = \frac{B}{A}x[n-1] - \frac{C}{A}x[n-2] + \frac{1}{A}(F[n] + F[n-1])$$

---

###### 4. Why this equation is "approximately correct" (Mathematical proof)

We will prove, using a Taylor expansion, that this difference equation obtained by the bilinear transformation converges with high accuracy to the solution of the original continuous-time differential equation (the local truncation error is $O(T^3)$).

We will verify the error of the trapezoidal rule for the time integral of the continuous function $f(t)$. Taylor expanding $f(t)$ around $t = nT$ gives:

$$f(t) = f[n] + \dot{f}[n](t - nT) + \frac{\ddot{f}[n]}{2}(t - nT)^2 + O((t - nT)^3)$$

The true integral of this from $(n-1)T$ to $nT$, $I_{true}$, is:

$$I_{true} = \int_{-T}^{0} \left( f[n] + \dot{f}[n]\tau + \frac{\ddot{f}[n]}{2}\tau^2 + \dots \right) d\tau = f[n]T - \frac{\dot{f}[n]}{2}T^2 + \frac{\ddot{f}[n]}{6}T^3 + O(T^4)$$

On the other hand, the approximation $I_{approx}$ using the bilinear transformation (trapezoidal rule) is obtained by using the equation $f[n-1] = f[n] - \dot{f}[n]T + \frac{\ddot{f}[n]}{2}T^2 - \dots$ obtained by Taylor expanding $f[n-1]$ in the reverse direction around $t = nT$:

$$I_{approx} = \frac{T}{2}(f[n] + f[n-1]) = \frac{T}{2} \left( 2f[n] - \dot{f}[n]T + \frac{\ddot{f}[n]}{2}T^2 \right) = f[n]T - \frac{\dot{f}[n]}{2}T^2 + \frac{\ddot{f}[n]}{4}T^3 + O(T^4)$$

When calculating the difference between the true value and the approximation (local truncation error $\epsilon_{local}$), the lower-order terms completely cancel each other out.

$$\epsilon_{local} = I_{true} - I_{approx} = \left( \frac{1}{6} - \frac{1}{4} \right) \ddot{f}[n]T^3 + O(T^4) = -\frac{1}{12}\ddot{f}[n]T^3 + O(T^4)$$

Since the error per step is $O(T^3)$ (order 3), the cumulative total truncation error up to the time limit $t$ ($t/T$ steps) is $O(T^2)$ (order 2).

</details>
