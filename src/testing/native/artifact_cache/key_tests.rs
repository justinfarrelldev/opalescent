//! Each specified artifact-key dimension is independently authoritative.
#![cfg(target_os = "linux")]
use super::*;

#[test]
fn all_abi_profile_compiler_target_configuration_and_source_inputs_change_keys() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project =
        NativeTestProject::load(&root, &[root.join("tests/addition.test.op")]).expect("project");
    let checked = super::super::checking::check_project(&project).expect("checked");
    let baseline = key(&inputs(&project, &checked).expect("inputs")).expect("key");
    let mutations: [fn(&mut Inputs); 10] = [
        |value| value.runtime_abi += 1,
        |value| value.compiler_binary.push('x'),
        |value| value.compiler_version = "different-version",
        |value| value.target.push_str("-different"),
        |value| value.profile = "different-profile",
        |value| value.testing_configuration = "different-testing-features",
        |value| {
            value.modules[0]
                .2
                .push_str("\ndouble Changed replaces changed from ./other as used by ./app\n");
        },
        |value| value.linker_version.push(1),
        |value| {
            value
                .tool_environment
                .push((OsString::from("CC"), Some(OsString::from("different"))));
        },
        |value| value.cases.clear(),
    ];
    for mutate in mutations {
        let mut changed = inputs(&project, &checked).expect("inputs");
        mutate(&mut changed);
        assert_ne!(baseline, key(&changed).expect("changed key"));
    }
}
