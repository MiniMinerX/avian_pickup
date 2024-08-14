use crate::prelude::*;

mod can_pull;
mod find_in_cone;
mod find_in_trace;

use self::{can_pull::*, find_in_cone::*, find_in_trace::*};

pub(super) fn plugin(app: &mut App) {
    app.observe(find_object)
        .get_schedule_mut(PhysicsSchedule)
        .unwrap()
        .add_systems(flush_pulling_state.in_set(AvianPickupSystem::ResetIdle));
}

#[derive(Debug, Event)]
pub(crate) struct PullObject;

/// Inspired by [`CWeaponPhysCannon::FindObject`](https://github.com/ValveSoftware/source-sdk-2013/blob/master/sp/src/game/server/hl2/weapon_physcannon.cpp#L2497)
fn find_object(
    trigger: Trigger<PullObject>,
    spatial_query: SpatialQuery,
    mut q_actor: Query<(
        &GlobalTransform,
        &AvianPickupActor,
        &mut AvianPickupActorState,
        &mut Cooldown,
    )>,
    q_collider: Query<&ColliderParent>,
    mut q_rigid_body: Query<(&RigidBody, &Mass, &mut ExternalImpulse, &GlobalTransform)>,
    q_transform: Query<&GlobalTransform>,
) {
    let actor_entity = trigger.entity();
    let (origin, config, mut state, mut cooldown) = q_actor.get_mut(actor_entity).unwrap();

    if !cooldown.right.finished() {
        return;
    }

    let origin = origin.compute_transform();
    let prop = find_prop_in_trace(&spatial_query, origin, config)
        .or_else(|| find_prop_in_cone(&spatial_query, origin, config, &q_transform));

    let Some(prop) = prop else {
        return;
    };

    // unwrap cannot fail: all colliders have a `ColliderParent`
    let rigid_body_entity = q_collider.get(prop.entity).unwrap().get();

    let Ok((&rigid_body, &mass, mut impulse, object_transform)) =
        q_rigid_body.get_mut(rigid_body_entity)
    else {
        // These components might not be present on non-dynamic rigid bodies
        return;
    };

    if !can_pull(rigid_body, mass, config) {
        return;
    }

    let can_hold = prop.toi <= config.trace_length;
    if can_hold {
        cooldown.hold();
        *state = AvianPickupActorState::Holding(prop.entity);
    } else {
        let object_transform = object_transform.compute_transform();
        let direction = origin.translation - object_transform.translation;
        let mass_adjustment = adjust_impulse_for_mass(mass);
        let pull_impulse = direction * config.pull_force * mass_adjustment;
        cooldown.pull();
        impulse.apply_impulse(pull_impulse);
        *state = AvianPickupActorState::Pulling(prop.entity);
    }
}

/// Taken from [this snippet](https://github.com/ValveSoftware/source-sdk-2013/blob/master/sp/src/game/server/hl2/weapon_physcannon.cpp#L2607-L2610)
fn adjust_impulse_for_mass(mass: Mass) -> f32 {
    if mass.0 < 50.0 {
        (mass.0 + 0.5) * (1.0 / 50.0)
    } else {
        1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]

struct Prop {
    pub entity: Entity,
    pub toi: f32,
}

fn flush_pulling_state(mut q_state: Query<(Mut<AvianPickupActorState>, &Cooldown)>) {
    for (mut state, cooldown) in q_state.iter_mut() {
        // Okay, so the basic idea is this:
        // Pulling happens in discrete impulses every n milliseconds.
        // New pulls happen regularly, but we should also reset to idle at some point.
        // Technically, we could reset to idle every frame and let it be overwritten by
        // a new pull, but that would mean that in the time between discrete
        // impulses, the state would be idle. That is technically true, but
        // probably not what a user observing the state component would expect.
        // So, instead we set it to idle only if the cooldown is finished.
        //
        // The reason we check for `is_changed` is that we run in `PostUpdate` by
        // default, so we would change a newly set `Pulling` state to `Idle`
        // immediately.
        if !state.is_changed()
            && matches!(state.as_ref(), AvianPickupActorState::Pulling(..))
            && cooldown.right.finished()
        {
            *state = AvianPickupActorState::Idle;
        }
    }
}