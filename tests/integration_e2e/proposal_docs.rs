use std::fs;
use std::path::Path;

#[test]
fn terminal_generated_testing_harness_proposal_file_exists_and_is_linked() {
    let proposal_path = Path::new(
        "stdlib-proposals/terminal-generated-testing/harness-injected-fake-backend/proposal.md",
    );
    assert!(
        proposal_path.exists(),
        "selected harness-injected fake-backend proposal file should exist"
    );

    let comparison =
        fs::read_to_string("stdlib-proposals/terminal-generated-testing/COMPARISON.md")
            .expect("terminal generated testing comparison should be readable");
    assert!(
        comparison.contains("harness-injected-fake-backend/proposal.md"),
        "comparison should link to the selected harness-injected fake-backend proposal"
    );

    let readme = fs::read_to_string("stdlib-proposals/README.md")
        .expect("stdlib proposals README should be readable");
    assert!(
        readme.contains("terminal-generated-testing/harness-injected-fake-backend/"),
        "stdlib proposals README should list the selected harness-injected fake-backend alternative"
    );
}
