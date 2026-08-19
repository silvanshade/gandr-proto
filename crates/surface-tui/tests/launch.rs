//! Observable TUI launch path.

#[cfg(test)]
mod tests
{
    use gandr_surface_tui::SMOKE_NOTE;
    use gandr_surface_tui::run_smoke;

    #[test]
    fn smoke_writes_the_launch_note()
    {
        let mut output = Vec::new();
        run_smoke(&mut output).expect("the smoke face must draw");
        assert_eq!(
            output.as_slice(),
            SMOKE_NOTE.as_bytes(),
            "the launch note is the smoke observable"
        );
    }
}
