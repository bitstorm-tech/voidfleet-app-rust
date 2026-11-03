//! Der Systemzeiger unter macOS direkt über AppKit.
//!
//! winit 0.30 nimmt ein eigenes Zeigerbild in Punkten statt Pixeln (auf Retina doppelt hochgezogen, unscharf;
//! rust-windowing/winit#4094) und kopiert sein Alpha unmultipliziert in die Bitmap (dunkler Saum um helle Kanten).
//! Darum baut dieses Modul den `NSCursor` selbst: ein `NSImage` in Punktgröße mit je einer Representation in
//! einfacher und doppelter Auflösung aus PNG-Daten, AppKit wählt je Bildschirm die passende. winit setzt seinen
//! Zeiger allein über Cursor-Rects (`resetCursorRects`); `disableCursorRects` schaltet die für das Fenster ab,
//! danach gilt `NSCursor::set`, bis ein anderes Fenster den Zeiger ändert, also nach jedem Eintritt der Maus und
//! jedem Fokuswechsel erneut ([`reapply`]). Bevys `CursorIcon` und `CursorOptions` wirken unter macOS damit nicht.

use std::cell::RefCell;

use bevy::{ecs::system::NonSendMarker, math::Vec2};
use objc2::{AllocAnyThread, MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSApplication, NSBitmapImageRep, NSCursor, NSImage};
use objc2_foundation::{NSData, NSPoint, NSSize};

thread_local! {
    /// Der gesetzte Zeiger; `NSCursor` gehört auf den Hauptthread, deshalb nicht in der ECS.
    static CURSOR: RefCell<Option<Retained<NSCursor>>> = const { RefCell::new(None) };
}

/// Baut den Zeiger aus den PNGs in einfacher und doppelter Auflösung, `points` Punkte groß mit `hotspot` in
/// Punkten, und setzt ihn. Läuft aus einem System mit `NonSendMarker`, also auf dem Hauptthread.
pub(super) fn install(pngs: [Vec<u8>; 2], points: f32, hotspot: Vec2) {
    let size = NSSize::new(points.into(), points.into());
    let image = NSImage::initWithSize(NSImage::alloc(), size);
    for png in pngs {
        let rep =
            NSBitmapImageRep::initWithData(NSBitmapImageRep::alloc(), &NSData::with_bytes(&png))
                .expect("PNG aus tiny-skia ist lesbar");
        rep.setSize(size);
        image.addRepresentation(&rep);
    }
    let cursor = NSCursor::initWithImage_hotSpot(
        NSCursor::alloc(),
        &image,
        NSPoint::new(hotspot.x.into(), hotspot.y.into()),
    );
    set(&cursor);
    CURSOR.with(|current| *current.borrow_mut() = Some(cursor));
}

/// Setzt den Zeiger erneut, nachdem ein anderes Fenster ihn geändert haben kann: wenn die Maus ins Fenster kommt
/// oder der Fokus wechselt.
pub(super) fn reapply(_main_thread: NonSendMarker) {
    CURSOR.with(|current| {
        if let Some(cursor) = &*current.borrow() {
            set(cursor);
        }
    });
}

/// Nimmt winit die Cursor-Rects der Fenster weg (jedes Mal, beim ersten Zeiger kann das Fenster noch fehlen) und
/// setzt den Zeiger.
fn set(cursor: &NSCursor) {
    let mtm =
        MainThreadMarker::new().expect("Systeme mit NonSendMarker laufen auf dem Hauptthread");
    for window in NSApplication::sharedApplication(mtm).windows().iter() {
        if window.areCursorRectsEnabled() {
            window.disableCursorRects();
        }
    }
    cursor.set();
}
