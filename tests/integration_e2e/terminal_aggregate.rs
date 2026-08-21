#[test]
fn terminal_aggregate_proposal_traceability_mentions_private_runtime_contract() {
    let chords = include_str!("../../stdlib-proposals/terminal-session-input/CHORDS.md");
    let example =
        include_str!("../../stdlib-proposals/terminal-session-input/configure_editor_chords.op");

    assert!(
        chords.contains("transactional affine aggregate rule"),
        "CHORDS.md should keep the private aggregate construction contract traceable"
    );
    assert!(
        chords.contains("Candidate -> Full after successful owned-registration retarget"),
        "CHORDS.md should name the retarget-to-commit transition"
    );
    assert!(
        example.contains("no fallible operation intervenes after retarget"),
        "configure_editor_chords.op should document the no-interposition rule"
    );
    assert!(
        example.contains("Partial construction instead uses reverse acquisition order"),
        "configure_editor_chords.op should document rollback cleanup ordering"
    );
}
