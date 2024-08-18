use crate::{prelude::*, verb::Dropping};

pub(super) fn plugin(app: &mut App) {
    app.get_schedule_mut(PhysicsSchedule)
        .unwrap()
        .add_systems(drop.in_set(AvianPickupSystem::HandleVerb));
}

fn drop(mut q_state: Query<(&mut AvianPickupActorState, &mut Cooldown), With<Dropping>>) {
    for (mut state, mut cooldown) in q_state.iter_mut() {
        if !cooldown.right.finished() {
            continue;
        }
        *state = AvianPickupActorState::Idle;
        info!("Drop!");
        cooldown.drop();
    }
}
