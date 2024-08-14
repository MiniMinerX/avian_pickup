use std::os::unix::raw::time_t;

use bevy::render::render_resource::encase::rts_array::Length;

use crate::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.get_schedule_mut(PhysicsSchedule)
        .unwrap()
        .add_systems(hold.in_set(AvianPickupSystem::HoldObject));
}

fn hold(q_actor: Query<(&AvianPickupActorState, &GlobalTransform)>) {
    for (&state, transform) in q_actor.iter() {
        let AvianPickupActorState::Holding(_entity) = state else {
            continue;
        };
        let _transform = transform.compute_transform();
        info!("Hold!")
    }
}

#[derive(Debug, Copy, Clone, Component)]
struct ShadowParams {
    target_position: Vec3,
    target_rotation: Quat,
    max_angular: f32,
    max_damp_angular: f32,
    max_speed: f32,
    max_damp_speed: f32,
    // damp_factor = 1
    // teleport_distance = 0
}

#[derive(Debug, Copy, Clone, Component)]
struct GrabParams {
    contact_amount: f32,
    time_to_arrive: f32,
    error_time: f32,
}

fn grabcontroller_simulate(
    time: Res<Time>,
    mut q_object: Query<(
        &ShadowParams,
        &Mass,
        &mut LinearVelocity,
        &mut AngularVelocity,
        &Position,
        &Rotation,
        &mut GrabParams,
    )>,
) {
    let dt = time.delta_seconds();
    for (shadow, mass, mut velocity, mut angvel, position, rotation, mut grab) in
        q_object.iter_mut()
    {
        // imo InContactWithHeavyObject will always be false,
        // as we are effectively asking "is the current object heavier than the
        // current object?"
        // TODO: make this smooth_nudge
        grab.contact_amount = grab.contact_amount.lerp(1.0, dt * 2.0);
        let mut shadow = *shadow;
        shadow.max_angular *= grab.contact_amount * grab.contact_amount * grab.contact_amount;

        grab.time_to_arrive = compute_shadow_control(
            &mut shadow,
            grab.time_to_arrive,
            dt,
            *position,
            *rotation,
            &mut velocity,
            &mut angvel,
        );

        // Slide along the current contact points to fix bouncing problems
        *velocity = phys_compute_slide_direction(*velocity, *angvel, *mass);
        grab.error_time += dt;
    }
}

fn compute_shadow_control(
    shadow: &mut ShadowParams,
    seconds_to_arrival: f32,
    dt: f32,
    position: Position,
    rotation: Rotation,
    velocity: &mut LinearVelocity,
    angvel: &mut AngularVelocity,
) -> f32 {
    compute_shadow_controller(
        shadow,
        position,
        rotation,
        velocity,
        angvel,
        seconds_to_arrival,
        dt,
    )
}

fn compute_shadow_controller(
    params: &mut ShadowParams,
    position: Position,
    rotation: Rotation,
    linear_velocity: &mut LinearVelocity,
    angular_velocity: &mut AngularVelocity,
    seconds_to_arrival: f32,
    dt: f32,
) -> f32 {
    let fraction = if seconds_to_arrival > 0.0 {
        (dt / seconds_to_arrival).min(1.0)
    } else {
        1.0
    };

    let seconds_to_arrival = (seconds_to_arrival - dt).max(0.0);
    if fraction <= 0.0 {
        return seconds_to_arrival;
    }

    let delta_position = params.target_position - position.0;
    // Teleport distance is always 0, so we don't care about that branch of the
    // code. That would be the only place where position and rotation are
    // mutated, so that means we get to use them immutably here!

    let inv_dt = dt.recip();
    let fraction_time = fraction * inv_dt;

    *linear_velocity = compute_controller(
        linear_velocity.0,
        delta_position,
        params.max_speed,
        params.max_damp_speed,
        fraction_time,
    )
    .into();

    // Don't think this is used? It at least doesn't appear in 2013's shadow params
    let _last_position = position.0 + linear_velocity.0 * dt;

    let delta_rotation = params.target_rotation * rotation.0.inverse();

    let delta_angles = delta_rotation.to_scaled_axis();
    *angular_velocity = compute_controller(
        angular_velocity.0,
        delta_angles,
        params.max_angular,
        params.max_damp_angular,
        fraction_time,
    )
    .into();

    seconds_to_arrival
}

fn phys_compute_slide_direction(
    velocity: LinearVelocity,
    _angular_velocity: AngularVelocity,
    _min_mass: Mass,
) -> LinearVelocity {
    // No need to return angular velocity, as we are not using it in the 2013 code

    // Sooooooo
    // The Jolt code depends on `CreatePhysicsSnapshot`, BUT
    // [that is actually not implemented](https://github.com/Joshua-Ashton/VPhysics-Jolt/blob/main/vphysics_jolt/vjolt_friction.cpp#L26)
    // Meanwhile, 2003's `CGrabController::Simulate` just runs
    // `ComputeShadowControl` and does not even have any
    // `PhysComputeSlideDirection` method. So I guess we don't need it? Jolt's
    // implementation has a somewhat unsure sounding comment about not needing
    // this either, but I guess we're good to go?
    velocity
}

fn compute_controller(
    mut velocity: Vec3,
    delta: Vec3,
    max_speed: f32,
    max_damp_speed: f32,
    scale_delta: f32,
) -> Vec3 {
    let current_speed_sq = velocity.length_squared();
    if current_speed_sq < 1e-6 {
        velocity = Vec3::ZERO;
    } else if max_damp_speed > 0.0 {
        // max_damp_speed = 4
        let mut acceleration_damping = velocity * -1.0; // vel = (8, 0, 0) -> accel_d = (-8, 0, 0)
        let speed = current_speed_sq.sqrt(); // speed = 8
        if speed > max_damp_speed {
            let some_factor_idk = max_damp_speed / speed; // some_fac = 4 / 8 = 0.5
            acceleration_damping *= some_factor_idk; // accel_d = (-4, 0, 0)
        }
        velocity += acceleration_damping; // vel = (4, 0, 0)
    }

    let mut acceleration = Vec3::ZERO;
    if max_speed > 0.0 {
        acceleration = delta * scale_delta; // accel = (8, 0, 0)
        let speed = delta.length() * scale_delta; // speed = 8
        if speed > max_speed {
            let some_factor_idk = max_speed / speed; // some_fac = 4 / 8 = 0.5
            acceleration *= some_factor_idk; // accel = (4, 0, 0)
        }
        velocity += acceleration; // vel = (4, 0, 0)
    }
    velocity
}

fn compute_controller_trimmed(
    mut velocity: Vec3,
    delta: Vec3,
    max_speed: f32,
    max_damp_speed: f32,
    scale_delta: f32,
) -> Vec3 {
    let current_speed_sq = velocity.length_squared();
    if current_speed_sq > (max_damp_speed * max_damp_speed) {
        let (dir, speed) = Dir3::new_and_length(velocity).unwrap();
        let new_speed = speed - max_damp_speed;
        velocity = dir * new_speed;
    } else {
        velocity = Vec3::ZERO;
    }

    if max_speed > 0.0 {
        let mut acceleration = delta * scale_delta;
        let accel_speed_sq = acceleration.length_squared();
        if accel_speed_sq > (max_speed * max_speed) {
            let norm = Dir3::new(acceleration).unwrap();
            acceleration = norm * max_speed;
        }
        velocity += acceleration;
    }
    velocity
}

fn compute_collider_no_damp(
    velocity: Vec3,
    delta: Vec3,
    max_speed: f32,
    max_damp_speed: f32,
    scale_delta: f32,
) -> Vec3 {
    if max_speed > 0.0 {
        let mut acceleration = delta * scale_delta;
        let accel_speed_sq = acceleration.length_squared();
        if accel_speed_sq > (max_speed * max_speed) {
            let norm = Dir3::new(acceleration).unwrap();
            acceleration = norm * max_speed;
        }
        acceleration
    } else {
        Vec3::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_compute_controller_same_as_trimmed() {
        for vel in 0..300 {
            for delta in 0..300 {
                let vel = Vec3::new(vel as f32, 0.0, 0.0);
                let delta = Vec3::new(delta as f32, 0.0, 0.0);

                let max_speed = 35.0;
                let max_damp_speed = 2.0 * max_speed;
                let scale_delta = 0.5;

                let orig = compute_controller(vel, delta, max_speed, max_damp_speed, scale_delta);
                let trimmed =
                    compute_controller_trimmed(vel, delta, max_speed, max_damp_speed, scale_delta);

                let diff = (orig - trimmed).length();
                if diff > 1e-5 {
                    panic!(
                        "Difference between compute_controller and compute_controller_trimmed: {diff}\n\
                        orig: {orig}, trimmed: {trimmed}\n\
                        Velocity: {vel}, Delta: {delta}, max_speed: {max_speed}, max_damp_speed: {max_damp_speed}, scale_delta: {scale_delta}"
                    );
                }
            }
        }
    }
}
