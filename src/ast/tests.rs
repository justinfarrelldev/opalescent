extern crate alloc;

use super::*;
use crate::token::Position;

fn dummy_span() -> Span {
    Span::single(Position::start())
}

fn dummy_node_id() -> NodeId {
    NodeId(0)
}

#[test]
fn test_literal_expr() {
    let expr = Expr::Literal {
        value: LiteralValue::Integer(42),
        span: dummy_span(),
        id: dummy_node_id(),
    };

    assert_eq!(expr.span(), dummy_span());
    assert_eq!(expr.node_id(), dummy_node_id());
}

#[test]
fn test_binary_expr() {
    let left = Box::new(Expr::Literal {
        value: LiteralValue::Integer(1),
        span: dummy_span(),
        id: dummy_node_id(),
    });

    let right = Box::new(Expr::Literal {
        value: LiteralValue::Integer(2),
        span: dummy_span(),
        id: dummy_node_id(),
    });

    let expr = Expr::Binary {
        left,
        operator: BinaryOp::Add,
        right,
        span: dummy_span(),
        id: dummy_node_id(),
    };

    assert_eq!(expr.span(), dummy_span());
    assert_eq!(expr.node_id(), dummy_node_id());
}

#[test]
fn test_statement_span_and_node_id() {
    let binding = LetBinding {
        name: "value".to_owned(),
        type_annotation: None,
        is_mutable: false,
        span: dummy_span(),
        id: dummy_node_id(),
    };

    let stmt = Stmt::Let {
        binding,
        initializer: None,
        span: dummy_span(),
        id: dummy_node_id(),
    };

    assert_eq!(stmt.span(), dummy_span());
    assert_eq!(stmt.node_id(), dummy_node_id());
}

#[test]
fn test_declaration_span_and_node_id() {
    let decl = Decl::Import {
        statement: ImportStatement {
            names: alloc::vec!["*".to_owned()],
            module: "./module".to_owned(),
        },
        items: alloc::vec![ImportItem::Glob { span: dummy_span() }],
        source: "./module".to_owned(),
        span: dummy_span(),
        id: dummy_node_id(),
        metadata: HotReloadMetadata::for_import(),
    };

    assert_eq!(decl.span(), dummy_span());
    assert_eq!(decl.node_id(), dummy_node_id());
}

#[test]
fn test_import_item_span_accessor() {
    let item = ImportItem::Named {
        name: "value".to_owned(),
        alias: Some("alias".to_owned()),
        span: dummy_span(),
    };

    assert_eq!(item.span(), dummy_span());
}
