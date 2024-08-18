use crate::{prelude::*, verb::Throwing};

pub(super) fn plugin(app: &mut App) {
    app.get_schedule_mut(PhysicsSchedule)
        .unwrap()
        .add_systems(throw.in_set(HandleVerbSystem::Throw));
}

fn throw(mut q_actor: Query<&mut Cooldown, With<Throwing>>) {
    for cooldown in q_actor.iter_mut() {
        if !cooldown.left.finished() {
            continue;
        }
        // Todo: cooldown.throw();
        info!("Throw!");
    }
}
