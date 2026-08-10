/// Small, explicit view DSL. It only expands to `Element` constructors; it
/// owns no state and performs no rendering.
#[macro_export]
macro_rules! view {
    (vertical [$($child:expr),* $(,)?]) => {
        $crate::view::Element::vertical([$($child),*])
    };
    (vertical_iter $children:expr) => {
        $crate::view::Element::vertical($children)
    };
    (horizontal [$($child:expr),* $(,)?]) => {
        $crate::view::Element::horizontal([$($child),*])
    };
    (slot $slot:expr) => {
        $crate::view::Element::slot($slot)
    };
}

/// Declarative layout-entry DSL. It expands directly to immutable
/// `LayoutEntry` constructors; allocation and terminal rendering remain
/// outside the macro.
#[macro_export]
macro_rules! layout_entries {
    (@entry fixed($slot:expr, $size:expr)) => {
        $crate::view::LayoutEntry::fixed($slot, $size)
    };
    (@entry grow($slot:expr, $min_size:expr)) => {
        $crate::view::LayoutEntry::grow($slot, $min_size)
    };
    ( $( $kind:ident($slot:expr, $size:expr) ),+ $(,)? ) => {
        [$( $crate::layout_entries!(@entry $kind($slot, $size)) ),+]
    };
}

/// Declare the stable component/slot/owner table as data instead of repeating
/// the same `ComponentSpec` constructor at every row.
#[macro_export]
macro_rules! component_specs {
    ($(($kind:ident, $slot:ident, $owner:ident)),+ $(,)?) => {
        [$(
            $crate::view::ComponentSpec {
                kind: $crate::view::ComponentKind::$kind,
                slot: $crate::view::Slot::$slot,
                owner: $crate::view::StateOwner::$owner,
            }
        ),+]
    };
}
