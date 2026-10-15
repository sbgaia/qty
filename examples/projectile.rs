//! A projectile in two dimensions: vectors of quantities, with the physics
//! checked by the compiler.
//!
//! Run with `cargo run --example projectile`.

use unitsnet_rs::*;

fn main() {
    let launch_speed = Speed::from_kilometers_per_hour(90.0);
    let launch_angle = Angle::from_degrees(35.0);
    let gravity = Vector2::<Acceleration>::from_base([0.0, -9.806_65]);

    // Simulate with small time steps (semi-implicit Euler).
    let dt = Duration::from_milliseconds(1.0);
    let mut position = Vector2::<Length>::from_base([0.0, 0.0]);
    let mut velocity: Vector2<Speed> = Vector2::from_polar(launch_speed, launch_angle);
    let mut time = Duration::from_seconds(0.0);
    let mut peak = Length::from_meters(0.0);
    loop {
        velocity += gravity * dt; // Acceleration · Duration = Speed
        let next = position + velocity * dt; // Speed · Duration = Length
        time += dt;
        peak = peak.max(next.y);
        if next.y < Length::from_meters(0.0) {
            break;
        }
        position = next;
    }

    // The closed-form answer, using the relations between quantities:
    // range = v² · sin(2θ) / g.
    let g = Acceleration::from_meters_per_second_squared(9.806_65);
    let v_squared: SpecificEnergy = launch_speed * launch_speed;
    let range = Length::from_base(v_squared.as_base() * (launch_angle * 2.0).sin() / g.as_base());

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
