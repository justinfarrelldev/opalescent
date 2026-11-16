//! Static dependency graph regression tests using resolved nominal identities.
use super::identity::{DeclarationIdentity, WorldIdentity};
use super::world_graph::{WorldGraph, WorldNode};

/// A resolved local declaration identity for graph-only unit fixtures.
fn world(name: &str) -> WorldIdentity {
    WorldIdentity(DeclarationIdentity {
        module: "tests/worlds.op".into(),
        name: name.into(),
    })
}

#[test]
fn diamond_initializes_once_and_independent_nodes_follow_declaration_order() {
    let graph = WorldGraph::new(vec![
        WorldNode {
            identity: world("Root"),
            dependencies: vec![],
        },
        WorldNode {
            identity: world("Independent"),
            dependencies: vec![],
        },
        WorldNode {
            identity: world("Left"),
            dependencies: vec![world("Root")],
        },
        WorldNode {
            identity: world("Right"),
            dependencies: vec![world("Root")],
        },
        WorldNode {
            identity: world("Top"),
            dependencies: vec![world("Left"), world("Right")],
        },
    ])
    .expect("valid graph");
    assert_eq!(
        graph
            .initialization_order(&[world("Top"), world("Independent")])
            .expect("order"),
        vec![
            world("Root"),
            world("Independent"),
            world("Left"),
            world("Right"),
            world("Top")
        ]
    );
    assert_eq!(
        graph
            .initialization_order(&[world("Top"), world("Root")])
            .expect("shared dependency"),
        vec![world("Root"), world("Left"), world("Right"), world("Top")]
    );
}

#[test]
fn cycles_report_the_complete_closed_cycle_path() {
    let error = WorldGraph::new(vec![
        WorldNode {
            identity: world("A"),
            dependencies: vec![world("B")],
        },
        WorldNode {
            identity: world("B"),
            dependencies: vec![world("C")],
        },
        WorldNode {
            identity: world("C"),
            dependencies: vec![world("A")],
        },
    ])
    .expect_err("cycle must fail compilation");
    assert_eq!(
        error.cycle_path(),
        Some([world("A"), world("B"), world("C"), world("A")].as_slice())
    );
}

#[test]
fn unresolved_and_duplicate_dependencies_are_rejected() {
    assert!(
        WorldGraph::new(vec![WorldNode {
            identity: world("A"),
            dependencies: vec![world("Missing")]
        }])
        .is_err()
    );
    assert!(
        WorldGraph::new(vec![WorldNode {
            identity: world("A"),
            dependencies: vec![world("A"), world("A")]
        }])
        .is_err()
    );
    assert!(
        WorldGraph::new(vec![
            WorldNode {
                identity: world("A"),
                dependencies: vec![]
            },
            WorldNode {
                identity: world("A"),
                dependencies: vec![]
            },
        ])
        .is_err()
    );
}
