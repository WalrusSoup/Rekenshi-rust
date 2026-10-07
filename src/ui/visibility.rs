//! Visibility policy shared by the native menu adapter and its regression tests.
#![forbid(unsafe_code)]

pub(super) fn visible_ancestors<T: Copy>(
    mut widget: Option<T>,
    mut inspect: impl FnMut(T) -> (bool, Option<T>),
) -> bool {
    if widget.is_none() {
        return false;
    }
    // Bound traversal so an unexpected hierarchy cannot stall the render loop.
    for _ in 0..64 {
        let Some(current) = widget else { return true };
        let (visible, parent) = inspect(current);
        if !visible {
            return false;
        }
        widget = parent;
    }
    widget.is_none()
}

#[cfg(test)]
mod tests {
    use super::visible_ancestors;

    #[test]
    fn closing_settings_hides_panel_even_when_options_tab_still_exists() {
        // Kenshi retains OptionsTab and hides its containing settings window.
        let tree = [(true, Some(1)), (true, Some(2)), (false, None)];
        assert!(!visible_ancestors(Some(0), |i| tree[i]));
    }

    #[test]
    fn visible_settings_and_destroyed_settings() {
        let tree = [(true, Some(1)), (true, None)];
        assert!(visible_ancestors(Some(0), |i| tree[i]));
        assert!(!visible_ancestors::<usize>(None, |_| unreachable!()));
    }

    #[test]
    fn hidden_tab_and_cyclic_ancestry_fail_closed() {
        assert!(!visible_ancestors(Some(0), |_| (false, None)));
        assert!(!visible_ancestors(Some(0), |i| (true, Some(i))));
    }
}
