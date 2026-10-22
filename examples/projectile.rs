//! A projectile in two dimensions: vectors of quantities, with the physics
//! checked by the compiler.
//!
//! Run with `cargo run --example projectile`.

use qty::*;

fn main() {
    let launch_speed: Speed = qty!(90 km/h);
    let launch_angle: Angle = qty!(35 deg);
    let g: Acceleration = qty!(9.806_65 m/s^2);
    let gravity = Vector2::new(Acceleration::ZERO, -g);

    // Simulate with small time steps (semi-implicit Euler).
    let dt: Duration = qty!(1 ms);
    let mut position = Vector2::new(Length::ZERO, Length::ZERO);
    let mut velocity: Vector2<Speed> = Vector2::from_polar(launch_speed, launch_angle);
    let mut time = Duration::ZERO;
    let mut peak = Length::ZERO;
    loop {
        velocity += gravity * dt; // Acceleration · Duration = Speed
        let next = position + velocity * dt; // Speed · Duration = Length
        time += dt;
        peak = peak.max(next.y);
        if next.y < Length::ZERO {
            break;
        }
        position = next;
    }

    // The closed-form answer, range = v² · sin(2θ) / g, with the plain
    // operators: v² is a `SpecificEnergy`, and over an acceleration a length.
    let range: Length = launch_speed.powi::<2>() * (launch_angle * 2.0).sin() / g;

    println!(
        "launch:     {:.1} at {:.0}",
        launch_speed.in_unit(SpeedUnit::MeterPerSecond),
        launch_angle.in_unit(AngleUnit::Degree)
    );
    println!("flight:     {:.2}", time.in_unit(DurationUnit::Second));
    println!("peak:       {:.2}", peak.in_unit(LengthUnit::Meter));
    println!(
        "range:      {:.2} (simulated)",
        position.x.in_unit(LengthUnit::Meter)
    );
    println!(
        "range:      {:.2} (formula)",
        range.in_unit(LengthUnit::Meter)
    );
    println!(
        "impact:     {:.1}",
        velocity.norm().in_unit(SpeedUnit::KilometerPerHour)
    );

    // Momentum of a 0.43 kg ball at impact: Mass · Vector2<Speed>.
    let momentum: Vector2<Impulse> = Mass::from_grams(430.0) * velocity;
    println!("momentum:   {:.2}", momentum);

    let error = (position.x - range).abs() / range;
    assert!(
        error < 0.01,
        "simulation differs from the formula by {error}"
    );
}
