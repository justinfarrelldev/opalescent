//! Structured test-module parser acceptance tests.
use crate::ast::testing::TestItem;
use crate::lexer::Lexer;
use crate::parser::Parser;

#[test]
fn parses_documented_scenarios_worlds_and_both_callback_layouts() {
    let source = "##\nDescription: isolated numbers\n##\nworld Numbers:\n    setup: f(): int64 =>\n        return 4\n\nscenario user's # numbers:\n    worlds: Numbers\n    kind: integration\n    it should use inline callbacks: f(ref numbers: Numbers): void =>\n        print(numbers.state)\n        return void\n    it should use indented callbacks:\n        f(ref numbers: Numbers): void =>\n            print(numbers.state)\n            return void\n";
    let (tokens, lex_errors) = Lexer::new_test_module(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (module, errors) = Parser::new(tokens).parse_test_module();
    assert!(errors.is_empty(), "{errors:?}");
    let module = module.expect("test module");
    assert_eq!(module.items.len(), 2);
    assert!(matches!(module.items[0], TestItem::World(_)));
    if let TestItem::World(ref world) = module.items[0] {
        assert_eq!(world.name, "Numbers");
        assert!(world.documentation.is_some());
        assert!(world.cleanup.is_none());
    }
    assert!(matches!(module.items[1], TestItem::Scenario(_)));
    if let TestItem::Scenario(ref scenario) = module.items[1] {
        assert_eq!(scenario.title.as_deref(), Some("user's # numbers"));
        assert_eq!(scenario.items.len(), 4);
    }
}

#[test]
fn rejects_malformed_worlds_callbacks_and_subject_expressions() {
    for source in [
        "world Missing:\n    cleanup: f(ref state: int64): void =>\n        return void\n",
        "world Duplicate:\n    setup: f(): int64 =>\n        return 1\n    setup: f(): int64 =>\n        return 2\n",
        "scenario invalid:\n    unit under test: add(2, 2)\n",
        "scenario invalid:\n    unit under test: 42\n",
        "scenario invalid:\n    kind: arbitrary\n",
        "scenario invalid:\n    it should 'interpolate {name}': f(): void =>\n        return void\n",
        "scenario invalid:\n    it should have two callbacks:\n        f(): void =>\n            return void\n        f(): void =>\n            return void\n",
    ] {
        let (tokens, lex_errors) = Lexer::new_test_module(source).tokenize();
        assert!(lex_errors.is_empty(), "{lex_errors:?}");
        let (module, errors) = Parser::new(tokens).parse_test_module();
        assert!(module.is_none(), "accepted {source}");
        assert!(!errors.is_empty(), "missing diagnostic for {source}");
    }
}

#[test]
fn hooks_inherit_no_local_variables_and_keep_typed_callbacks() {
    let source = "before each test:\n    f(): void =>\n        return void\nscenario nested metadata:\n    unit under test: arithmetic.add\n    worlds: Workspace\n    activate worlds: NetworkForbidden\n    scenario narrower:\n        worlds: none\n        after each test: f(): void errors inferred =>\n            return void\n        it should have no injections: f(): void =>\n            return void\n";
    let (tokens, lex_errors) = Lexer::new_test_module(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (module, errors) = Parser::new(tokens).parse_test_module();
    assert!(module.is_some(), "{errors:?}");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn parses_named_examples_and_explicit_test_support_imports() {
    let source = "import world Workspace as PrivateWorkspace from ./worlds\nimport double AtomicWrite from ./doubles\nimport type Input from ./input.types\nscenario inputs:\n    it should accept named data:\n        examples of Input:\n            example user's first\\: input:\n                text: 'one'\n            example second # input:\n                text: 'two'\n        f(ref example: Input): void =>\n            print(example.text)\n            return void\n";
    let (tokens, lex_errors) = Lexer::new_test_module(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (module, errors) = Parser::new(tokens).parse_test_module();
    assert!(errors.is_empty(), "{errors:?}");
    let module = module.expect("test module");
    assert!(matches!(module.items[0], TestItem::SupportImport(_)));
    assert!(matches!(module.items[3], TestItem::Scenario(_)));
    if let TestItem::Scenario(ref scenario) = module.items[3] {
        assert!(matches!(scenario.items[0], TestItem::Test(_)));
        if let TestItem::Test(ref test) = scenario.items[0] {
            let examples = test.examples.as_ref().expect("examples");
            assert_eq!(examples.examples.len(), 2);
            assert_eq!(examples.examples[0].title, "user's first: input");
            assert_eq!(examples.examples[1].title, "second # input");
        }
    }
}

#[test]
fn parses_token_bounded_assertions_without_splitting_string_contents() {
    let source = "scenario assertions:\n    it should evaluate expressions: f(): void =>\n        expect label('to be', 'not to contain') to be 'to be'\n        expect 3 to be greater than 2\n        expect 'hi' not to end with 'x'\n        expect ['a', 'b'] to contain exactly ['a', 'b'] in order\n        expect status to be the Written variant of EditorStatus\n        return void\n";
    let (tokens, lex_errors) = Lexer::new_test_module(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (module, errors) = Parser::new(tokens).parse_test_module();
    assert!(module.is_some(), "{errors:?}");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn parses_import_edge_double_and_parent_relative_imports() {
    let source = "import add from ../src/arithmetic\ndouble AtomicWrite replaces write_text_atomic_sync from standard\n    as used by ../src/buffer\nscenario counts:\n    it should record calls: f(): void =>\n        expect writer to have been called once\n        expect writer to have been called exactly 2 times\n        expect writer to have been called with ('path', 'contents')\n        expect writer to have never been called\n        return void\n";
    let (tokens, lex_errors) = Lexer::new_test_module(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (module, errors) = Parser::new(tokens).parse_test_module();
    assert!(module.is_some(), "{errors:?}");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn parses_double_configuration_and_polling_observations() {
    let source = "scenario configured writes:\n    it should control only the edge: f(): void =>\n        stub writer:\n            otherwise fail the test\n        stub writer('path', 'contents')\n            returns void\n        stub writer('other', 'contents')\n            fails with FilesystemFullError\n        stub writer using replacement\n        spy writer using original\n        verify writer\n        restore writer\n        expect eventually terminal_row(ref terminal, 11)\n            to contain '-- NORMAL --'\n            within 2 seconds\n        return void\n";
    let (tokens, lex_errors) = Lexer::new_test_module(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (module, errors) = Parser::new(tokens).parse_test_module();
    assert!(module.is_some(), "{errors:?}");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn recovered_callback_errors_never_produce_an_executable_test_module() {
    let source = "scenario bad callback:\n    it should reject malformed statements: f(): void =>\n        expect 1 to invent 2\n        return void\n";
    let (tokens, _) = Lexer::new_test_module(source).tokenize();
    let (module, errors) = Parser::new(tokens).parse_test_module();
    assert!(!errors.is_empty());
    assert!(
        module.is_none(),
        "a recovered callback error must invalidate the artifact"
    );
}

#[test]
fn parses_exclusion_keys_as_constant_scheduler_metadata() {
    let source = "scenario laboratory:\n    requires exclusive access to: 'device', 'account'\n    it should keep scheduling explicit: f(): void =>\n        return void\n";
    let (tokens, _) = Lexer::new_test_module(source).tokenize();
    let (module, errors) = Parser::new(tokens).parse_test_module();
    assert!(module.is_some(), "{errors:?}");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn parses_minimal_native_testing_project_fixture() {
    let source = include_str!("../../test-projects/native-testing-basic/tests/addition.test.op");
    let (tokens, lex_errors) = Lexer::new_test_module(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (module, errors) = Parser::new(tokens).parse_test_module();
    assert!(errors.is_empty(), "{errors:?}");
    let module = module.expect("fixture module");
    assert!(matches!(module.items[3], TestItem::Scenario(_)));
    if let TestItem::Scenario(ref scenario) = module.items[3] {
        assert!(scenario.documentation.is_some());
        assert!(matches!(scenario.items[4], TestItem::Test(_)));
        if let TestItem::Test(ref test) = scenario.items[4] {
            assert!(test.documentation.is_some());
            assert_eq!(
                test.examples
                    .as_ref()
                    .expect("named examples")
                    .examples
                    .len(),
                2
            );
        }
    }
}

#[test]
fn ordinary_parser_rejects_test_declarations() {
    let (tokens, _) =
        Lexer::new("world Numbers:\n    setup: f(): int64 =>\n        return 4\n").tokenize();
    let (program, errors) = Parser::new(tokens).parse();
    assert!(program.is_none());
    assert!(!errors.is_empty());
}
