//! Only chrome is tinted. Artwork, maps and coloured status fills keep their colours.
#![forbid(unsafe_code)]
/// Four consecutive floats in the fingerprinted MyGUI colour representation.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Colour(pub f32, pub f32, pub f32, pub f32);

/// Actual ResourceSkin names in Kenshi's kenshi_skins.xml. Templates expand into
/// these skins before the native initialization hook classifies each widget.
pub(super) fn is_surface(skin: &str) -> bool {
    matches!(
        skin,
        "Kenshi_NamePanelSkin"
            | "Kenshi_HealthPanelTopSkin"
            | "Kenshi_HealthPanelBottomSkin"
            | "Kenshi_SquadPanelSkin"
            | "Kenshi_SquadPanelHeaderSkin"
            | "Kenshi_InventoryPanelSkin"
            | "Kenshi_FloorPanelSkin"
            | "Kenshi_OrdersPanelSkin"
            | "Kenshi_SpeedPanelSkin"
            | "Kenshi_TimeMoneyPanelSkin"
            | "Kenshi_OrdersTextboxSkin"
            | "Kenshi_SpeedTextboxSkin"
            | "Kenshi_GenericWindowSkin"
            | "Kenshi_GenericWindowHeaderSkin"
            | "Kenshi_InventoryWindowSkin"
            | "Kenshi_InventoryWindowHeaderSkin"
            | "Kenshi_ExtendedStatsPanelSkin"
            | "Kenshi_StatsPanelSkin"
            | "Kenshi_FloatingPanelSkin"
            | "Kenshi_FloatinglPanelSmallSkin"
            | "Kenshi_FloatingPanelThinSkin"
            | "Kenshi_GenericTextBoxSkin"
            | "Kenshi_GenericTextBoxBorderlessSkin"
            | "Kenshi_GenericTextBoxFlatSkin"
            | "Kenshi_GenericTextBoxFlatSmallSkin"
            | "Kenshi_GenericInputBoxSkin"
            | "Kenshi_FrameSkin"
            | "Kenshi_PortraitFrameSkin"
            | "Kenshi_BuildPanelSkin"
            | "Kenshi_SpeechPanelSkin"
            | "Kenshi_SelectionPanel"
            | "Kenshi_MessageSkin"
            | "Kenshi_InventorySlotSkin"
            | "Kenshi_ListBoxItemSkin"
            | "Kenshi_Button1Skin"
            | "Kenshi_SquadTabButton1Skin"
            | "Kenshi_ExtendedStatsButton"
            | "Kenshi_HScrollerButtonSkin"
            | "Kenshi_VScrollerButtonSkin"
            | "Kenshi_BuildButtonSkin"
            | "Kenshi_BehaviorButtonSkin"
            | "Kenshi_BehaviorSliderSkin"
            | "Kenshi_SliderSkin"
            | "Kenshi_ScrollerVerticalSkin"
            | "Kenshi_ScrollerHorizontalSkin"
            | "Kenshi_ProgressBarEmptySkin"
            | "Kenshi_TickButton1Skin"
            | "Kenshi_TickBoxSkin"
    )
}

pub(super) fn minimum_brightness(skin: &str) -> f32 {
    // These atlas tiles include dark checkbox glyphs as well as their frames.
    // A lighter control face retains the original tick's contrast.
    if matches!(skin, "Kenshi_TickButton1Skin" | "Kenshi_TickBoxSkin") {
        0.50
    } else {
        0.
    }
}

pub(super) fn surface(colour: Colour, brightness: f32) -> Colour {
    // A slightly cooler tint counters the beige atlas without removing its rust,
    // edge wear, transparency or nine-slice geometry. Preserve native alpha.
    Colour(
        colour.0 * brightness * 0.88,
        colour.1 * brightness,
        colour.2 * brightness * 1.12,
        colour.3,
    )
}
pub(super) fn text(colour: Colour) -> Colour {
    let channels = [colour.0, colour.1, colour.2];
    if channels.iter().any(|v| !v.is_finite()) {
        return colour;
    }
    let high = channels.into_iter().fold(0., f32::max);
    let low = channels.into_iter().fold(1., f32::min);
    if high >= 0.70 {
        return colour;
    }
    if high - low <= 0.18 {
        // Keep muted/disabled neutrals distinguishable from the near-black body text.
        let value = if high >= 0.55 { 0.76 } else { 0.93 };
        Colour(value, value * 0.98, value * 0.93, colour.3)
    } else {
        // Keep warnings red, selected labels gold and status text green.
        let scale = 0.90 / high.max(0.01);
        Colour(
            colour.0 * scale,
            colour.1 * scale,
            colour.2 * scale,
            colour.3,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hud_and_windows_are_chrome_but_artwork_and_status_fills_are_not() {
        for name in [
            "Kenshi_NamePanelSkin",
            "Kenshi_HealthPanelBottomSkin",
            "Kenshi_SquadPanelSkin",
            "Kenshi_InventoryWindowSkin",
            "Kenshi_GenericWindowHeaderSkin",
            "Kenshi_Button1Skin",
        ] {
            assert!(is_surface(name));
        }
        for name in [
            "ImageBox",
            "WhiteSkin",
            "PanelEmpty",
            "Kenshi_LifeBarSkin",
            "Kenshi_LifeBarHurtSkin",
            "Kenshi_LifeBarHealedSkin",
            "Kenshi_ProgressBarFilledSkin",
            "Kenshi_TimeSpeedButton1Skin",
            "unknown_mod_skin",
        ] {
            assert!(!is_surface(name));
        }
    }
    #[test]
    fn theme_preserves_alpha_bright_text_and_status_hues() {
        let dark = text(Colour(0.078, 0.031, 0.024, 0.4));
        assert!(dark.0 > 0.9 && dark.3 == 0.4);
        let bright = text(Colour(0.8, 0.4, 0.1, 0.7));
        assert_eq!(
            [bright.0, bright.1, bright.2, bright.3],
            [0.8, 0.4, 0.1, 0.7]
        );
        let red = text(Colour(0.5, 0., 0., 0.6));
        assert!(red.0 > 0.8 && red.1 == 0. && red.2 == 0. && red.3 == 0.6);
        let panel = surface(Colour(1., 1., 1., 0.3), 0.28);
        assert!(panel.0 < panel.1 && panel.1 < panel.2 && panel.3 == 0.3);
    }
}
