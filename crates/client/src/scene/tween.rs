//! Tweens im Bildtakt (Port von `scene/tween.ts`), gestreckt um die eingestellte Animationsgeschwindigkeit.
//!
//! Ein Tween ist eine eigene Entity mit [`Tween<C>`]: Er ruft jedes Bild seine Funktion mit dem geglätteten Fortschritt
//! 0..1 auf der Komponente `C` seines Ziels auf und löst am Ende [`TweenFinished`] auf sich selbst aus, bevor er
//! verschwindet. So laufen beliebig viele Tweens auf derselben Komponente, und wer auf das Ende wartet, hängt einen
//! Observer an den Tween:
//!
//! ```ignore
//! commands
//!     .spawn(Tween::new(ship, 0.4, move |transform: &mut Transform, t| {
//!         transform.translation = from.lerp(to, t);
//!     }).with_ease(EaseFunction::CubicOut))
//!     .observe(|_: On<TweenFinished>| { /* weiter */ });
//! ```
//!
//! Für eigene Komponenten meldet ein Plugin den Typ mit [`TweenAppExt::add_tween_target`] an; `Transform` ist schon
//! angemeldet. Welcher Geschwindigkeitsregler gilt, bestimmt [`AnimationClock::scope`] beim Start des Tweens (die
//! Rundenwiedergabe schaltet für Angriffe um) oder [`Tween::in_scope`]. Solange [`AnimationClock::skip`] gilt, springen
//! alle Tweens sofort an ihr Ende.

use bevy::{ecs::component::Mutable, prelude::*};

use crate::settings::Settings;

/// Welcher Regler der Animationsgeschwindigkeit gilt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnimationScope {
    /// Schüsse im Kampf.
    Attack,
    /// Flüge der Schiffe: Deploy-Einflug und Lane-Wechsel.
    Movement,
    /// Alle übrigen Animationen.
    #[default]
    General,
}

/// Gemeinsamer Takt aller Animationen.
#[derive(Resource, Default, Debug)]
pub struct AnimationClock {
    /// Regler, den neue Tweens ohne eigene Angabe nehmen.
    pub scope: AnimationScope,
    /// Animationen überspringen: Laufende Tweens springen an ihr Ende, neue enden sofort; Sounds und Funk schweigen.
    pub skip: bool,
}

impl Settings {
    /// Streckungsfaktor der Dauern für einen Regler.
    pub fn time_scale(&self, scope: AnimationScope) -> f32 {
        match scope {
            AnimationScope::Attack => self.attack_speed,
            AnimationScope::Movement => self.movement_speed,
            AnimationScope::General => self.animation_speed,
        }
        .time_scale()
    }
}

/// Was ein Tween je Bild mit dem geglätteten Fortschritt auf seinem Ziel tut.
type Apply<C> = Box<dyn Fn(&mut C, f32) + Send + Sync>;

/// Ein laufender Tween auf der Komponente `C` der Entity `target`.
#[derive(Component)]
pub struct Tween<C: Component<Mutability = Mutable>> {
    target: Entity,
    /// Dauer in Sekunden vor der Streckung.
    duration: f32,
    elapsed: f32,
    scope: Option<AnimationScope>,
    ease: EaseFunction,
    apply: Apply<C>,
}

impl<C: Component<Mutability = Mutable>> Tween<C> {
    /// Linearer Tween über `seconds` (vor der Streckung durch die Animationsgeschwindigkeit).
    pub fn new(
        target: Entity,
        seconds: f32,
        apply: impl Fn(&mut C, f32) + Send + Sync + 'static,
    ) -> Self {
        Self {
            target,
            duration: seconds,
            elapsed: 0.0,
            scope: None,
            ease: EaseFunction::Linear,
            apply: Box::new(apply),
        }
    }

    /// Glättet den Fortschritt, etwa mit `EaseFunction::CubicOut` (`easeOutCubic`) oder `SineInOut` (`easeInOutSine`).
    pub fn with_ease(mut self, ease: EaseFunction) -> Self {
        self.ease = ease;
        self
    }

    /// Nimmt diesen Regler statt des gerade geltenden.
    pub fn in_scope(mut self, scope: AnimationScope) -> Self {
        self.scope = Some(scope);
        self
    }

    fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).min(1.0)
        }
    }
}

/// Ausgelöst auf der Tween-Entity, wenn ihr Tween fertig ist (auch beim Überspringen); danach verschwindet sie.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct TweenFinished {
    /// Die Tween-Entity.
    pub entity: Entity,
    /// Das Ziel des Tweens.
    pub target: Entity,
}

/// Meldet Komponenten an, auf denen Tweens laufen können.
pub trait TweenAppExt {
    fn add_tween_target<C: Component<Mutability = Mutable>>(&mut self) -> &mut Self;
}

impl TweenAppExt for App {
    fn add_tween_target<C: Component<Mutability = Mutable>>(&mut self) -> &mut Self {
        self.add_systems(Update, advance_tweens::<C>.in_set(TweenSystems))
    }
}

/// Die Systeme, die Tweens weiterschalten; wer ihr Ergebnis im selben Bild lesen will, ordnet sich danach.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TweenSystems;

pub(super) struct TweenPlugin;

impl Plugin for TweenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AnimationClock>()
            .add_tween_target::<Transform>();
    }
}

fn advance_tweens<C: Component<Mutability = Mutable>>(
    mut commands: Commands,
    mut tweens: Query<(Entity, &mut Tween<C>)>,
    mut targets: Query<&mut C>,
    time: Res<Time>,
    clock: Res<AnimationClock>,
    settings: Res<Settings>,
) {
    for (entity, mut tween) in &mut tweens {
        let Ok(mut component) = targets.get_mut(tween.target) else {
            // Das Ziel ist verschwunden (etwa ein entsorgtes Schiff): Der Tween endet still.
            commands.entity(entity).despawn();
            continue;
        };
        let scope = *tween.scope.get_or_insert(clock.scope);
        tween.elapsed = if clock.skip {
            tween.duration
        } else {
            tween.elapsed + time.delta_secs() / settings.time_scale(scope)
        };
        let progress = tween.progress();
        (tween.apply)(&mut component, tween.ease.sample_clamped(progress));
        if progress >= 1.0 {
            commands.trigger(TweenFinished {
                entity,
                target: tween.target,
            });
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    #[derive(Component, Default)]
    struct Value(f32);

    #[derive(Resource, Default)]
    struct Finished(u32);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TweenPlugin))
            .add_tween_target::<Value>()
            .insert_resource(Settings::default())
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                100,
            )))
            .init_resource::<Finished>()
            .add_observer(|_: On<TweenFinished>, mut finished: ResMut<Finished>| finished.0 += 1);
        app
    }

    /// Ein Tween über 1 s läuft mit „Normal“ (Faktor 1,25) 1,25 s und meldet sein Ende genau einmal.
    #[test]
    fn runs_stretched_and_finishes() {
        let mut app = app();
        let target = app.world_mut().spawn(Value::default()).id();
        app.world_mut()
            .spawn(Tween::new(target, 1.0, |value: &mut Value, t| value.0 = t));
        // Das erste Update hat noch keine Zeitdifferenz.
        for _ in 0..11 {
            app.update();
        }
        let value = app.world().get::<Value>(target).unwrap().0;
        assert!((value - 0.8).abs() < 1e-4, "{value}");
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(app.world().get::<Value>(target).unwrap().0, 1.0);
        assert_eq!(app.world().resource::<Finished>().0, 1);
    }

    #[test]
    fn skip_ends_immediately() {
        let mut app = app();
        app.world_mut().resource_mut::<AnimationClock>().skip = true;
        let target = app.world_mut().spawn(Value::default()).id();
        app.world_mut()
            .spawn(Tween::new(target, 10.0, |value: &mut Value, t| value.0 = t));
        app.update();
        assert_eq!(app.world().get::<Value>(target).unwrap().0, 1.0);
        assert_eq!(app.world().resource::<Finished>().0, 1);
    }
}
