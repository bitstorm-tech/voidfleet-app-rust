//! Animationen als aufrufbare Bausteine mit Fertig-Rückmeldung (statt der Promises des Originals).
//!
//! Jede Animation ist eine eigene Entity, ein „Auftrag“: Eine Komponente beschreibt, was geschehen soll (etwa
//! [`crate::scene::effects::Bolt`] oder [`crate::scene::motion::Arrive`]), [`Animation`] zählt ihre Zeit im Takt der
//! Animationsgeschwindigkeit. Ist sie abgelaufen, löst sie [`AnimationFinished`] auf sich selbst aus und verschwindet
//! samt allem, was an ihr hängt. Wer auf das Ende wartet, hängt einen Observer an den Auftrag:
//!
//! ```ignore
//! commands
//!     .spawn(Bolt { from, to, color })
//!     .observe(|_: On<AnimationFinished>, mut commands: Commands| {
//!         commands.spawn(Impact::new(to, from, color));
//!     });
//! ```
//!
//! Ein Auftrag, der erst auf etwas warten muss (ein Schiff, dessen Modell noch lädt), bekommt seine [`Animation`]
//! erst, wenn er beginnt. Ein Auftrag nur aus [`Animation`] ist eine Pause.

use bevy::prelude::*;

use super::tween::{AnimationClock, AnimationScope};
use crate::settings::Settings;

/// Die Uhr eines Auftrags.
#[derive(Component, Clone, Copy, Debug)]
pub struct Animation {
    /// Sekunden vor der Streckung durch die Animationsgeschwindigkeit.
    duration: f32,
    elapsed: f32,
    /// Zuwachs von `elapsed` in diesem Bild.
    delta: f32,
    scope: Option<AnimationScope>,
}

impl Animation {
    /// Läuft `seconds` (vor der Streckung) im Regler, der beim Start gilt.
    pub fn new(seconds: f32) -> Self {
        Self {
            duration: seconds,
            elapsed: 0.0,
            delta: 0.0,
            scope: None,
        }
    }

    /// Nimmt diesen Regler statt des gerade geltenden.
    pub fn in_scope(mut self, scope: AnimationScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Fortschritt 0..1.
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).min(1.0)
        }
    }

    /// Verstrichene Sekunden (vor der Streckung).
    pub fn seconds(&self) -> f32 {
        self.elapsed
    }

    /// Sekunden (vor der Streckung) seit dem letzten Bild.
    pub fn delta(&self) -> f32 {
        self.delta
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn is_done(&self) -> bool {
        self.progress() >= 1.0
    }
}

/// Ausgelöst auf dem Auftrag, wenn seine Animation fertig ist (auch beim Überspringen); danach verschwindet er.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct AnimationFinished {
    pub entity: Entity,
}

/// Reihenfolge im Bild: erst läuft die Uhr, dann setzen die Aufträge ihr Bild, zuletzt melden die fertigen.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnimationSystems {
    Advance,
    Apply,
    Finish,
}

pub(super) struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                AnimationSystems::Advance,
                AnimationSystems::Apply,
                AnimationSystems::Finish,
            )
                .chain(),
        )
        .add_systems(Update, advance.in_set(AnimationSystems::Advance))
        .add_systems(Update, finish.in_set(AnimationSystems::Finish));
    }
}

fn advance(
    mut animations: Query<&mut Animation>,
    time: Res<Time>,
    clock: Res<AnimationClock>,
    settings: Res<Settings>,
) {
    for mut animation in &mut animations {
        let scope = *animation.scope.get_or_insert(clock.scope);
        let before = animation.elapsed;
        animation.elapsed = if clock.skip {
            animation.duration
        } else {
            (before + time.delta_secs() / settings.time_scale(scope)).min(animation.duration)
        };
        animation.delta = animation.elapsed - before;
    }
}

fn finish(mut commands: Commands, animations: Query<(Entity, &Animation)>) {
    for (entity, animation) in &animations {
        if animation.is_done() {
            commands.trigger(AnimationFinished { entity });
            commands.entity(entity).despawn();
        }
    }
}

/// `easeInCubic`.
pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

/// `easeOutCubic`.
pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// `easeInOutSine`.
pub fn ease_in_out_sine(t: f32) -> f32 {
    -((std::f32::consts::PI * t).cos() - 1.0) / 2.0
}

/// Zufallszahl zwischen `min` und `max`.
pub fn random_between((min, max): (f32, f32)) -> f32 {
    min + rand::random::<f32>() * (max - min)
}

/// Zufällige Richtung im Kegel um `axis` mit dem halben Öffnungswinkel `half_angle`, gleichverteilt über den Raumwinkel.
pub fn random_in_cone(axis: Vec3, half_angle: f32) -> Vec3 {
    let helper = if axis.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let side = helper.cross(axis).normalize();
    let up = axis.cross(side);
    let cos = 1.0 - rand::random::<f32>() * (1.0 - half_angle.cos());
    let sin = (1.0 - cos * cos).max(0.0).sqrt();
    let around = rand::random::<f32>() * std::f32::consts::TAU;
    axis * cos + side * sin * around.cos() + up * sin * around.sin()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    #[derive(Resource, Default)]
    struct Finished(u32);

    #[test]
    fn finishes_once_and_despawns() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AnimationPlugin))
            .insert_resource(Settings::default())
            .init_resource::<AnimationClock>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                100,
            )))
            .init_resource::<Finished>()
            .add_observer(|_: On<AnimationFinished>, mut finished: ResMut<Finished>| {
                finished.0 += 1
            });
        let job = app.world_mut().spawn(Animation::new(0.5)).id();
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(app.world().resource::<Finished>().0, 1);
        assert!(app.world().get_entity(job).is_err());
    }

    #[test]
    fn cone_stays_inside() {
        for _ in 0..100 {
            let direction = random_in_cone(Vec3::Y, 0.3);
            assert!((direction.length() - 1.0).abs() < 1e-4);
            assert!(direction.angle_between(Vec3::Y) <= 0.3 + 1e-4);
        }
    }
}
