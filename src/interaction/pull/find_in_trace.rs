use super::Prop;
use crate::{math::METERS_PER_INCH, prelude::*};

/// Inspired by [`CWeaponPhysCannon::FindObjectTrace`](https://github.com/ValveSoftware/source-sdk-2013/blob/master/mp/src/game/server/hl2/weapon_physcannon.cpp#L2470)
pub(super) fn find_prop_in_trace(
    spatial_query: &SpatialQuery,
    origin: Transform,
    config: &AvianPickupActor,
    q_collider: &Query<&Position, Without<Sensor>>,
) -> Option<Prop> {
    const MAGIC_FACTOR_ASK_VALVE: f32 = 4.0;
    // trace_length already has `METERS_PER_INCH` baked in by being in SI units,
    // so no need to multiply the magic factor by `METERS_PER_INCH` here
    let test_length = config.trace_length * MAGIC_FACTOR_ASK_VALVE;
    let hit = spatial_query.cast_ray_predicate(
        origin.translation,
        origin.forward(),
        test_length,
        true,
        &config.prop_filter,
        &|entity| q_collider.contains(entity),
    );

    if let Some(hit) = hit {
        Prop {
            entity: hit.entity,
            toi: hit.time_of_impact,
        }
        .into()
    } else {
        let fake_aabb_because_parry_cannot_do_aabb_casts =
            Cuboid::from_size(Vec3::splat(MAGIC_FACTOR_ASK_VALVE * METERS_PER_INCH * 2.)).into();
        let hit = spatial_query.cast_shape(
            &fake_aabb_because_parry_cannot_do_aabb_casts,
            origin.translation,
            origin.rotation,
            origin.forward(),
            test_length,
            false,
            &config.prop_filter,
        );
        if let Some(hit) = hit {
            Prop {
                entity: hit.entity,
                toi: hit.time_of_impact,
            }
            .into()
        } else {
            None
        }
    }
}
