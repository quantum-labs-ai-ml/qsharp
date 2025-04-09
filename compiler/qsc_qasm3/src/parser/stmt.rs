// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#[cfg(test)]
pub(crate) mod tests;

use qsc_data_structures::span::Span;
use std::rc::Rc;

use super::{
    completion::word_kinds::WordKinds,
    error::{Error, ErrorKind},
    expr::{self, designator, gate_operand, indexed_identifier},
    prim::{self, barrier, many, opt, recovering, recovering_semi, recovering_token, seq, shorten},
    Result,
};
use crate::{
    keyword::Keyword,
    lex::{cooked::Type, Delim, TokenKind},
};

use super::ast::{
    list_from_iter, AccessControl, AliasDeclStmt, AngleType, Annotation, ArrayBaseTypeKind,
    ArrayReferenceType, ArrayType, ArrayTypedParameter, AssignOpStmt, AssignStmt, BarrierStmt,
    BitType, Block, BoxStmt, BreakStmt, CalibrationGrammarStmt, CalibrationStmt, Cast,
    ClassicalDeclarationStmt, ComplexType, ConstantDeclStmt, ContinueStmt, DefCalStmt, DefStmt,
    DelayStmt, EndStmt, EnumerableSet, Expr, ExprKind, ExprStmt, ExternDecl, ExternParameter,
    FloatType, ForStmt, FunctionCall, GPhase, GateCall, GateModifierKind, GateOperand,
    IODeclaration, IOKeyword, Ident, Identifier, IfStmt, IncludeStmt, IndexElement, IndexExpr,
    IndexSetItem, IndexedIdent, IntType, List, LiteralKind, MeasureArrowStmt, Pragma,
    QuantumGateDefinition, QuantumGateModifier, QuantumTypedParameter, QubitDeclaration,
    RangeDefinition, ResetStmt, ReturnStmt, ScalarType, ScalarTypeKind, ScalarTypedParameter, Stmt,
    StmtKind, SwitchCase, SwitchStmt, TypeDef, TypedParameter, UIntType, WhileLoop,
};
use super::{prim::token, ParserContext};

/// Our implementation differs slightly from the grammar in
/// that we accumulate annotations and append them to the next
/// consecutive statement.
///
/// Grammar:
/// ```g4
/// pragma
/// | annotation* (
///     aliasDeclarationStatement
///     | assignmentStatement
///     | barrierStatement
///     | boxStatement
///     | breakStatement
///     | calStatement
///     | calibrationGrammarStatement
///     | classicalDeclarationStatement
///     | constDeclarationStatement
///     | continueStatement
///     | defStatement
///     | defcalStatement
///     | delayStatement
///     | endStatement
///     | expressionStatement
///     | externStatement
///     | forStatement
///     | gateCallStatement
///     | gateStatement
///     | ifStatement
///     | includeStatement
///     | ioDeclarationStatement
///     | measureArrowAssignmentStatement
///     | oldStyleDeclarationStatement
///     | quantumDeclarationStatement
///     | resetStatement
///     | returnStatement
///     | switchStatement
///     | whileStatement
/// )
/// ```
pub(super) fn parse(s: &mut ParserContext) -> Result<Stmt> {
    let lo = s.peek().span.lo;
    if let Some(pragma) = opt(s, parse_pragma)? {
        return Ok(Stmt {
            span: s.span(lo),
            annotations: [].into(),
            kind: Box::new(StmtKind::Pragma(pragma)),
        });
    }
    let attrs = many(s, parse_annotation)?;

    let kind = if token(s, TokenKind::Semicolon).is_ok() {
        if attrs.is_empty() {
            let err_item = default(s.span(lo));
            s.push_error(Error::new(ErrorKind::EmptyStatement(err_item.span)));
            return Ok(err_item);
        }
        let lo = attrs.iter().map(|a| a.span.lo).min().unwrap_or_default();
        let hi = attrs.iter().map(|a| a.span.hi).max().unwrap_or_default();
        let err_item = default(Span { lo, hi });
        s.push_error(Error::new(ErrorKind::FloatingAnnotation(err_item.span)));
        return Ok(err_item);
    } else if let Some(decl) = opt(s, parse_gatedef)? {
        decl
    } else if let Some(decl) = opt(s, parse_def)? {
        decl
    } else if let Some(include) = opt(s, parse_include)? {
        include
    } else if let Some(ty) = opt(s, scalar_or_array_type)? {
        disambiguate_type(s, ty)?
    } else if let Some(decl) = opt(s, parse_constant_classical_decl)? {
        decl
    } else if let Some(decl) = opt(s, parse_quantum_decl)? {
        decl
    } else if let Some(decl) = opt(s, parse_io_decl)? {
        decl
    } else if let Some(decl) = opt(s, qreg_decl)? {
        decl
    } else if let Some(decl) = opt(s, creg_decl)? {
        decl
    } else if let Some(decl) = opt(s, parse_extern)? {
        decl
    } else if let Some(switch) = opt(s, parse_switch_stmt)? {
        StmtKind::Switch(switch)
    } else if let Some(stmt) = opt(s, parse_if_stmt)? {
        StmtKind::If(stmt)
    } else if let Some(stmt) = opt(s, parse_for_stmt)? {
        StmtKind::For(stmt)
    } else if let Some(stmt) = opt(s, parse_while_loop)? {
        StmtKind::WhileLoop(stmt)
    } else if let Some(stmt) = opt(s, parse_return)? {
        stmt
    } else if let Some(stmt) = opt(s, parse_continue_stmt)? {
        StmtKind::Continue(stmt)
    } else if let Some(stmt) = opt(s, parse_break_stmt)? {
        StmtKind::Break(stmt)
    } else if let Some(stmt) = opt(s, parse_end_stmt)? {
        StmtKind::End(stmt)
    } else if let Some(indexed_ident) = opt(s, indexed_identifier)? {
        disambiguate_ident(s, indexed_ident)?
    } else if let Some(stmt_kind) = opt(s, parse_gate_call_stmt)? {
        stmt_kind
    } else if let Some(stmt) = opt(s, |s| parse_expression_stmt(s, None))? {
        StmtKind::ExprStmt(stmt)
    } else if let Some(alias) = opt(s, parse_alias_stmt)? {
        StmtKind::Alias(alias)
    } else if let Some(stmt) = opt(s, parse_box)? {
        StmtKind::Box(stmt)
    } else if let Some(stmt) = opt(s, parse_calibration_grammar_stmt)? {
        StmtKind::CalibrationGrammar(stmt)
    } else if let Some(stmt) = opt(s, parse_defcal_stmt)? {
        StmtKind::DefCal(stmt)
    } else if let Some(stmt) = opt(s, parse_cal)? {
        StmtKind::Cal(stmt)
    } else if let Some(stmt) = opt(s, parse_barrier)? {
        StmtKind::Barrier(stmt)
    } else if let Some(stmt) = opt(s, parse_delay)? {
        StmtKind::Delay(stmt)
    } else if let Some(stmt) = opt(s, parse_reset)? {
        StmtKind::Reset(stmt)
    } else if let Some(stmt) = opt(s, parse_measure_stmt)? {
        StmtKind::Measure(stmt)
    } else {
        return if attrs.is_empty() {
            Err(Error::new(ErrorKind::Rule(
                "statement",
                s.peek().kind,
                s.peek().span,
            )))
        } else {
            let span = attrs.last().expect("there is at least one annotation").span;
            Err(Error::new(ErrorKind::FloatingAnnotation(span)))
        };
    };

    Ok(Stmt {
        span: s.span(lo),
        annotations: attrs.into_boxed_slice(),
        kind: Box::new(kind),
    })
}

/// This helper function allows us to disambiguate between
/// non-constant declarations and cast expressions when
/// reading a `TypeDef`.
fn disambiguate_type(s: &mut ParserContext, ty: TypeDef) -> Result<StmtKind> {
    let lo = ty.span().lo;
    if matches!(s.peek().kind, TokenKind::Identifier) {
        Ok(parse_non_constant_classical_decl(s, ty, lo)?)
    } else {
        token(s, TokenKind::Open(Delim::Paren))?;
        let arg = expr::expr(s)?;
        recovering_token(s, TokenKind::Close(Delim::Paren));
        let cast_expr = Expr {
            span: s.span(ty.span().lo),
            kind: Box::new(ExprKind::Cast(Cast {
                span: s.span(ty.span().lo),
                ty,
                arg,
            })),
        };
        Ok(StmtKind::ExprStmt(parse_expression_stmt(
            s,
            Some(cast_expr),
        )?))
    }
}

/// This helper function allows us to disambiguate between
/// assignments, assignment operations, gate calls, and
/// `expr_stmts` beginning with an ident or a function call
/// when reading an `Ident`.
fn disambiguate_ident(s: &mut ParserContext, indexed_ident: IndexedIdent) -> Result<StmtKind> {
    let lo = indexed_ident.span.lo;
    if s.peek().kind == TokenKind::Eq {
        s.advance();
        let expr = expr::expr_or_measurement(s)?;
        recovering_semi(s);
        Ok(StmtKind::Assign(AssignStmt {
            span: s.span(lo),
            lhs: indexed_ident,
            rhs: expr,
        }))
    } else if let TokenKind::BinOpEq(op) = s.peek().kind {
        s.advance();
        let op = expr::closed_bin_op(op);
        let expr = expr::expr_or_measurement(s)?;
        recovering_semi(s);
        Ok(StmtKind::AssignOp(AssignOpStmt {
            span: s.span(lo),
            op,
            lhs: indexed_ident,
            rhs: expr,
        }))
    } else if s.peek().kind == TokenKind::Open(Delim::Paren) {
        if !indexed_ident.indices.is_empty() {
            s.push_error(Error::new(ErrorKind::Convert(
                "Ident",
                "IndexedIdent",
                indexed_ident.span,
            )));
        }

        let ident = indexed_ident.name;

        s.advance();
        let (args, _) = seq(s, expr::expr)?;
        token(s, TokenKind::Close(Delim::Paren))?;

        let funcall = Expr {
            span: s.span(lo),
            kind: Box::new(ExprKind::FunctionCall(FunctionCall {
                span: s.span(lo),
                name: ident,
                args: args.into_iter().map(Box::new).collect(),
            })),
        };

        let expr = expr::expr_with_lhs(s, funcall)?;

        Ok(parse_gate_call_with_expr(s, expr)?)
    } else {
        let kind = if indexed_ident.indices.is_empty() {
            ExprKind::Ident(indexed_ident.name)
        } else {
            if indexed_ident.indices.len() > 1 {
                s.push_error(Error::new(ErrorKind::MultipleIndexOperators(
                    indexed_ident.span,
                )));
            }

            ExprKind::IndexExpr(IndexExpr {
                span: indexed_ident.span,
                collection: Expr {
                    span: indexed_ident.name.span,
                    kind: Box::new(ExprKind::Ident(indexed_ident.name)),
                },
                index: *indexed_ident.indices[0].clone(),
            })
        };

        let expr = Expr {
            span: indexed_ident.span,
            kind: Box::new(kind),
        };

        Ok(parse_gate_call_with_expr(s, expr)?)
    }
}

#[allow(clippy::vec_box)]
pub(super) fn parse_many(s: &mut ParserContext) -> Result<Vec<Stmt>> {
    many(s, |s| {
        recovering(s, default, &[TokenKind::Semicolon], |s| {
            parse_block_or_stmt(s)
        })
    })
}

/// Grammar: `LBRACE statementOrScope* RBRACE`.
pub(super) fn parse_block(s: &mut ParserContext) -> Result<Block> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Open(Delim::Brace))?;
    let stmts = barrier(s, &[TokenKind::Close(Delim::Brace)], parse_many)?;
    recovering_token(s, TokenKind::Close(Delim::Brace));
    Ok(Block {
        span: s.span(lo),
        stmts: list_from_iter(stmts),
    })
}

#[allow(clippy::unnecessary_box_returns)]
fn default(span: Span) -> Stmt {
    Stmt {
        span,
        annotations: Vec::new().into_boxed_slice(),
        kind: Box::new(StmtKind::Err),
    }
}

/// Grammar: `AnnotationKeyword RemainingLineContent?`.
pub fn parse_annotation(s: &mut ParserContext) -> Result<Box<Annotation>> {
    let lo = s.peek().span.lo;
    s.expect(WordKinds::Annotation);

    let token = s.peek();
    let pat = &['\t', ' '];
    let parts: Vec<&str> = if token.kind == TokenKind::Annotation {
        let lexeme = s.read();
        s.advance();
        // remove @
        // split lexeme at first space/tab collecting each side
        shorten(1, 0, lexeme).splitn(2, pat).collect()
    } else {
        return Err(Error::new(ErrorKind::Rule(
            "annotation",
            token.kind,
            token.span,
        )));
    };

    let identifier = parts.first().map_or_else(
        || {
            Err(Error::new(ErrorKind::Rule(
                "annotation",
                token.kind,
                token.span,
            )))
        },
        |s| Ok(Into::<Rc<str>>::into(*s)),
    )?;

    if identifier.is_empty() {
        s.push_error(Error::new(ErrorKind::Rule(
            "annotation missing identifier",
            token.kind,
            token.span,
        )));
    }

    // remove any leading whitespace from the value side
    let value = parts
        .get(1)
        .map(|s| Into::<Rc<str>>::into(s.trim_start_matches(pat)));

    Ok(Box::new(Annotation {
        span: s.span(lo),
        identifier,
        value,
    }))
}

/// Grammar: `INCLUDE StringLiteral SEMICOLON`.
fn parse_include(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Include))?;
    let next = s.peek();

    let lit = expr::lit(s)?;
    recovering_semi(s);

    if let Some(lit) = lit {
        if let LiteralKind::String(v) = lit.kind {
            return Ok(StmtKind::Include(IncludeStmt {
                span: s.span(lo),
                filename: v.to_string(),
            }));
        }
    };
    Err(Error::new(ErrorKind::Rule(
        "string literal",
        next.kind,
        next.span,
    )))
}

/// Grammar: `PRAGMA RemainingLineContent`.
fn parse_pragma(s: &mut ParserContext) -> Result<Pragma> {
    let lo = s.peek().span.lo;
    s.expect(WordKinds::Pragma);

    let token = s.peek();

    let parts: Vec<&str> = if token.kind == TokenKind::Pragma {
        let lexeme = s.read();
        s.advance();
        // remove pragma keyword and any leading whitespace
        // split lexeme at first space/tab collecting each side
        let pat = &['\t', ' '];
        shorten(6, 0, lexeme)
            .trim_start_matches(pat)
            .splitn(2, pat)
            .collect()
    } else {
        return Err(Error::new(ErrorKind::Rule(
            "pragma", token.kind, token.span,
        )));
    };

    let identifier = parts.first().map_or_else(
        || {
            Err(Error::new(ErrorKind::Rule(
                "pragma", token.kind, token.span,
            )))
        },
        |s| Ok(Into::<Rc<str>>::into(*s)),
    )?;

    if identifier.is_empty() {
        s.push_error(Error::new(ErrorKind::Rule(
            "pragma missing identifier",
            token.kind,
            token.span,
        )));
    }
    let value = parts.get(1).map(|s| Into::<Rc<str>>::into(*s));

    Ok(Pragma {
        span: s.span(lo),
        identifier,
        value,
    })
}

/// Grammar: `EXTERN Identifier LPAREN externArgumentList? RPAREN returnSignature? SEMICOLON`.
fn parse_extern(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(crate::keyword::Keyword::Extern))?;
    let ident = Box::new(prim::ident(s)?);
    token(s, TokenKind::Open(Delim::Paren))?;
    let (params, _) = seq(s, extern_arg_def)?;
    token(s, TokenKind::Close(Delim::Paren))?;
    let return_type = opt(s, return_sig)?;
    recovering_semi(s);
    let kind = StmtKind::ExternDecl(ExternDecl {
        span: s.span(lo),
        ident,
        params: list_from_iter(params),
        return_type,
    });
    Ok(kind)
}

/// Grammar: `DEF Identifier LPAREN argumentDefinitionList? RPAREN returnSignature? scope`.
fn parse_def(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(crate::keyword::Keyword::Def))?;
    let name = Box::new(prim::ident(s)?);
    token(s, TokenKind::Open(Delim::Paren))?;
    let (exprs, _) = seq(s, arg_def)?;
    token(s, TokenKind::Close(Delim::Paren))?;
    let return_type = opt(s, return_sig)?.map(Box::new);
    let body = parse_block(s)?;
    let kind = StmtKind::Def(DefStmt {
        span: s.span(lo),
        name,
        params: list_from_iter(exprs),
        body,
        return_type,
    });
    Ok(kind)
}

/// Grammar: `scalarType | arrayReferenceType | CREG designator?`.
fn extern_arg_def(s: &mut ParserContext) -> Result<ExternParameter> {
    let lo = s.peek().span.lo;

    let kind = if let Ok(ty) = scalar_type(s) {
        ExternParameter::Scalar(ty, s.span(lo))
    } else if let Ok(ty) = array_reference_ty(s) {
        ExternParameter::ArrayReference(ty, s.span(lo))
    } else if let Ok(size) = extern_creg_type(s) {
        let ty = ScalarType {
            span: s.span(lo),
            kind: ScalarTypeKind::Bit(BitType {
                size,
                span: s.span(lo),
            }),
        };
        ExternParameter::Scalar(ty, s.span(lo))
    } else {
        return Err(Error::new(ErrorKind::Rule(
            "extern argument definition",
            s.peek().kind,
            s.peek().span,
        )));
    };
    Ok(kind)
}

/// Grammar:
/// ```g4
/// scalarType Identifier
/// | qubitType Identifier
/// | (CREG | QREG) Identifier designator?
/// | arrayReferenceType Identifier
/// ```
fn arg_def(s: &mut ParserContext) -> Result<TypedParameter> {
    let lo = s.peek().span.lo;

    let kind = if let Ok(ty) = scalar_type(s) {
        let ident = prim::ident(s)?;
        TypedParameter::Scalar(ScalarTypedParameter {
            span: s.span(lo),
            ty: Box::new(ty),
            ident,
        })
    } else if let Ok(size) = qubit_type(s) {
        let ident = prim::ident(s)?;
        TypedParameter::Quantum(QuantumTypedParameter {
            span: s.span(lo),
            size,
            ident,
        })
    } else if let Ok((ident, size)) = creg_type(s) {
        let ty = ScalarType {
            span: s.span(lo),
            kind: ScalarTypeKind::Bit(BitType {
                size,
                span: s.span(lo),
            }),
        };
        TypedParameter::Scalar(ScalarTypedParameter {
            span: s.span(lo),
            ty: Box::new(ty),
            ident,
        })
    } else if let Ok((ident, size)) = qreg_type(s) {
        TypedParameter::Quantum(QuantumTypedParameter {
            span: s.span(lo),
            size,
            ident,
        })
    } else if let Ok(ty) = array_reference_ty(s) {
        let ident = prim::ident(s)?;
        TypedParameter::ArrayReference(ArrayTypedParameter {
            span: s.span(lo),
            ty: Box::new(ty),
            ident,
        })
    } else {
        return Err(Error::new(ErrorKind::Rule(
            "argument definition",
            s.peek().kind,
            s.peek().span,
        )));
    };
    Ok(kind)
}

/// Grammar:
/// `(READONLY | MUTABLE) ARRAY LBRACKET scalarType COMMA (expressionList | DIM EQUALS expression) RBRACKET`.
fn array_reference_ty(s: &mut ParserContext) -> Result<ArrayReferenceType> {
    let lo = s.peek().span.lo;

    let mutability = if token(s, TokenKind::Keyword(crate::keyword::Keyword::ReadOnly)).is_ok() {
        AccessControl::ReadOnly
    } else if token(s, TokenKind::Keyword(crate::keyword::Keyword::Mutable)).is_ok() {
        AccessControl::Mutable
    } else {
        let token = s.peek();
        return Err(Error::new(ErrorKind::Rule(
            "array reference declaration",
            token.kind,
            token.span,
        )));
    };
    token(s, TokenKind::Type(Type::Array))?;
    token(s, TokenKind::Open(Delim::Bracket))?;
    let base_type = array_base_type(s)?;
    token(s, TokenKind::Comma)?;

    let dimensions = if token(s, TokenKind::Dim).is_ok() {
        token(s, TokenKind::Eq)?;
        vec![expr::expr(s)?]
    } else {
        expr::expr_list(s)?
    };

    token(s, TokenKind::Close(Delim::Bracket))?;
    Ok(ArrayReferenceType {
        span: s.span(lo),
        mutability,
        base_type,
        dimensions: list_from_iter(dimensions),
    })
}

/// Grammar: `ARROW scalarType`.
fn return_sig(s: &mut ParserContext) -> Result<ScalarType> {
    token(s, TokenKind::Arrow)?;
    scalar_type(s)
}

/// Grammar:
/// `GATE Identifier (LPAREN params=identifierList? RPAREN)? qubits=identifierList scope`.
fn parse_gatedef(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(crate::keyword::Keyword::Gate))?;
    let ident = Box::new(prim::ident(s)?);
    let params = opt(s, gate_params)?.unwrap_or_else(Vec::new);
    let (qubits, _) = seq(s, prim::ident)?;
    let body = Box::new(parse_block(s)?);
    Ok(StmtKind::QuantumGateDefinition(QuantumGateDefinition {
        span: s.span(lo),
        ident,
        params: list_from_iter(params),
        qubits: list_from_iter(qubits),
        body,
    }))
}

fn gate_params(s: &mut ParserContext<'_>) -> Result<Vec<Ident>> {
    token(s, TokenKind::Open(Delim::Paren))?;
    let (params, _) = seq(s, prim::ident)?;
    token(s, TokenKind::Close(Delim::Paren))?;
    Ok(params)
}

/// Grammar: `RETURN (expression | measureExpression)? SEMICOLON`.
fn parse_return(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(crate::keyword::Keyword::Return))?;
    let expr = opt(s, expr::expr_or_measurement)?.map(Box::new);
    recovering_semi(s);
    Ok(StmtKind::Return(ReturnStmt {
        span: s.span(lo),
        expr,
    }))
}

/// Grammar: `qubitType Identifier SEMICOLON`.
fn parse_quantum_decl(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;
    let size = qubit_type(s)?;
    let ty_span = s.span(lo);
    let ident = prim::ident(s)?;

    recovering_semi(s);
    Ok(StmtKind::QuantumDecl(QubitDeclaration {
        span: s.span(lo),
        ty_span,
        qubit: ident,
        size,
    }))
}

/// Grammar: `QUBIT designator?`.
fn qubit_type(s: &mut ParserContext<'_>) -> Result<Option<Expr>> {
    token(s, TokenKind::Keyword(crate::keyword::Keyword::Qubit))?;
    let size = opt(s, designator)?;
    Ok(size)
}

/// Grammar: `(INPUT | OUTPUT) (scalarType | arrayType) Identifier SEMICOLON`.
fn parse_io_decl(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;

    let kind = if token(s, TokenKind::Keyword(crate::keyword::Keyword::Input)).is_ok() {
        IOKeyword::Input
    } else if token(s, TokenKind::Keyword(crate::keyword::Keyword::Output)).is_ok() {
        IOKeyword::Output
    } else {
        let token = s.peek();
        return Err(Error::new(ErrorKind::Rule(
            "io declaration",
            token.kind,
            token.span,
        )));
    };

    let ty = scalar_or_array_type(s)?;

    let ident = Box::new(prim::ident(s)?);
    recovering_semi(s);
    let decl = IODeclaration {
        span: s.span(lo),
        io_identifier: kind,
        ty,
        ident,
    };
    Ok(StmtKind::IODeclaration(decl))
}

/// Grammar `(scalarType | arrayType)`.
pub fn scalar_or_array_type(s: &mut ParserContext) -> Result<TypeDef> {
    if let Ok(v) = scalar_type(s) {
        return Ok(TypeDef::Scalar(v));
    }
    if let Ok(v) = array_type(s) {
        return Ok(TypeDef::Array(v));
    }
    Err(Error::new(ErrorKind::Rule(
        "scalar or array type",
        s.peek().kind,
        s.peek().span,
    )))
}

/// Grammar: `(scalarType | arrayType) Identifier (EQUALS declarationExpression)? SEMICOLON`.
fn parse_non_constant_classical_decl(
    s: &mut ParserContext,
    ty: TypeDef,
    lo: u32,
) -> Result<StmtKind> {
    let identifier = prim::ident(s)?;
    let init_expr = if s.peek().kind == TokenKind::Eq {
        s.advance();
        Some(Box::new(expr::declaration_expr(s)?))
    } else {
        None
    };
    recovering_semi(s);
    let decl = ClassicalDeclarationStmt {
        span: s.span(lo),
        ty: Box::new(ty),
        identifier,
        init_expr,
    };

    Ok(StmtKind::ClassicalDecl(decl))
}

/// Grammar: `CONST scalarType Identifier EQUALS declarationExpression SEMICOLON`.
fn parse_constant_classical_decl(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Const))?;
    let ty = scalar_or_array_type(s)?;
    let identifier = Box::new(prim::ident(s)?);
    token(s, TokenKind::Eq)?;
    let init_expr = expr::const_declaration_expr(s)?;
    recovering_semi(s);
    let decl = ConstantDeclStmt {
        span: s.span(lo),
        ty,
        identifier,
        init_expr,
    };

    Ok(StmtKind::ConstDecl(decl))
}

/// The Spec and the grammar differ in the base type for arrays. We followed the Spec.
/// Grammar: `ARRAY LBRACKET arrayBaseType COMMA expressionList RBRACKET`.
pub(super) fn array_type(s: &mut ParserContext) -> Result<ArrayType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::Array))?;
    token(s, TokenKind::Open(Delim::Bracket))?;
    let kind = array_base_type(s)?;
    token(s, TokenKind::Comma)?;
    let expr_list = expr::expr_list(s)?;
    token(s, TokenKind::Close(Delim::Bracket))?;

    Ok(ArrayType {
        base_type: kind,
        span: s.span(lo),
        dimensions: list_from_iter(expr_list),
    })
}

/// The Spec for 3.0, main Spec, and the grammar differ in the base type for arrays.
/// We followed the main Spec.
/// Grammar:
/// | INT designator?
/// | UINT designator?
/// | FLOAT designator?
/// | ANGLE designator?
/// | BOOL
/// | DURATION
/// | COMPLEX (LBRACKET scalarType RBRACKET)?
/// Reference: <https://openqasm.com/language/types.html#arrays>.
pub(super) fn array_base_type(s: &mut ParserContext) -> Result<ArrayBaseTypeKind> {
    if let Ok(v) = array_angle_type(s) {
        return Ok(v);
    }
    if let Ok(v) = array_bool_type(s) {
        return Ok(v);
    }
    if let Ok(v) = array_int_type(s) {
        return Ok(v);
    }
    if let Ok(v) = array_uint_type(s) {
        return Ok(v);
    }
    if let Ok(v) = array_float_type(s) {
        return Ok(v);
    }
    if let Ok(v) = array_complex_type(s) {
        return Ok(v);
    }
    if let Ok(v) = array_duration_type(s) {
        return Ok(v);
    }

    Err(Error::new(ErrorKind::Rule(
        "array type",
        s.peek().kind,
        s.peek().span,
    )))
}

/// Grammar:
/// ```g4
/// BIT designator?
/// | INT designator?
/// | UINT designator?
/// | FLOAT designator?
/// | ANGLE designator?
/// | BOOL
/// | DURATION
/// | STRETCH
/// | COMPLEX (LBRACKET scalarType RBRACKET)?
/// ```
pub(super) fn scalar_type(s: &mut ParserContext) -> Result<ScalarType> {
    if let Ok(v) = scalar_bit_type(s) {
        return Ok(v);
    }
    if let Ok(v) = scalar_angle_type(s) {
        return Ok(v);
    }
    if let Ok(v) = scalar_bool_type(s) {
        return Ok(v);
    }
    if let Ok(v) = scalar_int_type(s) {
        return Ok(v);
    }
    if let Ok(v) = scalar_uint_type(s) {
        return Ok(v);
    }
    if let Ok(v) = scalar_float_type(s) {
        return Ok(v);
    }
    if let Ok(v) = scalar_complex_type(s) {
        return Ok(v);
    }
    if let Ok(v) = scalar_duration_type(s) {
        return Ok(v);
    }
    if let Ok(v) = scalar_stretch_type(s) {
        return Ok(v);
    }
    Err(Error::new(ErrorKind::Rule(
        "scalar type",
        s.peek().kind,
        s.peek().span,
    )))
}

fn creg_decl(s: &mut ParserContext) -> Result<StmtKind> {
    let lo: u32 = s.peek().span.lo;
    let (identifier, size) = creg_type(s)?;
    recovering_semi(s);
    Ok(StmtKind::ClassicalDecl(ClassicalDeclarationStmt {
        span: s.span(lo),
        ty: Box::new(TypeDef::Scalar(ScalarType {
            span: s.span(lo),
            kind: ScalarTypeKind::Bit(BitType {
                size,
                span: s.span(lo),
            }),
        })),
        identifier,
        init_expr: None,
    }))
}

fn qreg_decl(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;
    let (identifier, size) = qreg_type(s)?;
    recovering_semi(s);
    Ok(StmtKind::QuantumDecl(QubitDeclaration {
        span: s.span(lo),
        ty_span: s.span(lo),
        qubit: identifier,
        size,
    }))
}

fn extern_creg_type(s: &mut ParserContext) -> Result<Option<Expr>> {
    token(s, TokenKind::Keyword(crate::keyword::Keyword::CReg))?;
    let size = opt(s, designator)?;
    Ok(size)
}

fn creg_type(s: &mut ParserContext) -> Result<(Ident, Option<Expr>)> {
    token(s, TokenKind::Keyword(crate::keyword::Keyword::CReg))?;
    let name = prim::ident(s)?;
    let size = opt(s, designator)?;
    Ok((name, size))
}

fn qreg_type(s: &mut ParserContext) -> Result<(Ident, Option<Expr>)> {
    token(s, TokenKind::Keyword(crate::keyword::Keyword::QReg))?;
    let name = prim::ident(s)?;
    let size = opt(s, designator)?;
    Ok((name, size))
}

fn scalar_bit_type(s: &mut ParserContext) -> Result<ScalarType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::Bit))?;
    let size = opt(s, designator)?;
    Ok(ScalarType {
        span: s.span(lo),
        kind: ScalarTypeKind::Bit(BitType {
            size,
            span: s.span(lo),
        }),
    })
}

fn scalar_int_type(s: &mut ParserContext) -> Result<ScalarType> {
    let lo = s.peek().span.lo;
    let ty = int_type(s)?;
    Ok(ScalarType {
        span: s.span(lo),
        kind: ScalarTypeKind::Int(ty),
    })
}

fn array_int_type(s: &mut ParserContext) -> Result<ArrayBaseTypeKind> {
    let ty = int_type(s)?;
    Ok(ArrayBaseTypeKind::Int(ty))
}

fn int_type(s: &mut ParserContext) -> Result<IntType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::Int))?;
    let size = opt(s, designator)?;
    Ok(IntType {
        size,
        span: s.span(lo),
    })
}
fn scalar_uint_type(s: &mut ParserContext) -> Result<ScalarType> {
    let lo = s.peek().span.lo;
    let ty = uint_type(s)?;
    Ok(ScalarType {
        span: s.span(lo),
        kind: ScalarTypeKind::UInt(ty),
    })
}

fn array_uint_type(s: &mut ParserContext) -> Result<ArrayBaseTypeKind> {
    let ty = uint_type(s)?;
    Ok(ArrayBaseTypeKind::UInt(ty))
}

fn uint_type(s: &mut ParserContext) -> Result<UIntType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::UInt))?;
    let size = opt(s, designator)?;
    Ok(UIntType {
        size,
        span: s.span(lo),
    })
}

fn scalar_float_type(s: &mut ParserContext) -> Result<ScalarType> {
    let lo = s.peek().span.lo;
    let ty = float_type(s)?;
    Ok(ScalarType {
        span: s.span(lo),
        kind: ScalarTypeKind::Float(ty),
    })
}

fn array_float_type(s: &mut ParserContext) -> Result<ArrayBaseTypeKind> {
    let ty = float_type(s)?;
    Ok(ArrayBaseTypeKind::Float(ty))
}

fn float_type(s: &mut ParserContext) -> Result<FloatType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::Float))?;
    let size = opt(s, designator)?;
    Ok(FloatType {
        span: s.span(lo),
        size,
    })
}

fn scalar_angle_type(s: &mut ParserContext) -> Result<ScalarType> {
    let lo = s.peek().span.lo;
    let ty = angle_type(s)?;
    Ok(ScalarType {
        span: s.span(lo),
        kind: ScalarTypeKind::Angle(ty),
    })
}

fn array_angle_type(s: &mut ParserContext) -> Result<ArrayBaseTypeKind> {
    let ty = angle_type(s)?;
    Ok(ArrayBaseTypeKind::Angle(ty))
}

fn angle_type(s: &mut ParserContext) -> Result<AngleType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::Angle))?;
    let size = opt(s, designator)?;
    Ok(AngleType {
        size,
        span: s.span(lo),
    })
}

fn scalar_bool_type(s: &mut ParserContext) -> Result<ScalarType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::Bool))?;
    Ok(ScalarType {
        span: s.span(lo),
        kind: ScalarTypeKind::BoolType,
    })
}

fn array_bool_type(s: &mut ParserContext) -> Result<ArrayBaseTypeKind> {
    token(s, TokenKind::Type(Type::Bool))?;
    Ok(ArrayBaseTypeKind::BoolType)
}

fn scalar_duration_type(s: &mut ParserContext) -> Result<ScalarType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::Duration))?;
    Ok(ScalarType {
        span: s.span(lo),
        kind: ScalarTypeKind::Duration,
    })
}

fn array_duration_type(s: &mut ParserContext) -> Result<ArrayBaseTypeKind> {
    token(s, TokenKind::Type(Type::Duration))?;
    Ok(ArrayBaseTypeKind::Duration)
}

fn scalar_stretch_type(s: &mut ParserContext) -> Result<ScalarType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::Stretch))?;
    Ok(ScalarType {
        span: s.span(lo),
        kind: ScalarTypeKind::Stretch,
    })
}

pub(super) fn scalar_complex_type(s: &mut ParserContext) -> Result<ScalarType> {
    let lo = s.peek().span.lo;

    let ty = complex_type(s)?;
    Ok(ScalarType {
        span: s.span(lo),
        kind: ScalarTypeKind::Complex(ty),
    })
}

pub(super) fn array_complex_type(s: &mut ParserContext) -> Result<ArrayBaseTypeKind> {
    let ty = complex_type(s)?;
    Ok(ArrayBaseTypeKind::Complex(ty))
}

pub(super) fn complex_type(s: &mut ParserContext) -> Result<ComplexType> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Type(Type::Complex))?;

    let subty = opt(s, complex_subtype)?;
    Ok(ComplexType {
        base_size: subty,
        span: s.span(lo),
    })
}

pub(super) fn complex_subtype(s: &mut ParserContext) -> Result<FloatType> {
    token(s, TokenKind::Open(Delim::Bracket))?;
    let ty = float_type(s)?;
    token(s, TokenKind::Close(Delim::Bracket))?;
    Ok(ty)
}

/// The Language Spec and the grammar for switch statements disagree.
/// We followed the Spec when writing the parser.
/// Reference: <https://openqasm.com/language/classical.html#the-switch-statement>.
pub fn parse_switch_stmt(s: &mut ParserContext) -> Result<SwitchStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Switch))?;

    // Controlling expression.
    token(s, TokenKind::Open(Delim::Paren))?;
    let controlling_expr = expr::expr(s)?;
    recovering_token(s, TokenKind::Close(Delim::Paren));

    // Open cases bracket.
    token(s, TokenKind::Open(Delim::Brace))?;

    // Cases.
    let lo_cases = s.peek().span.lo;
    let cases = list_from_iter(many(s, case_stmt)?);
    if cases.is_empty() {
        s.push_error(Error::new(ErrorKind::MissingSwitchCases(s.span(lo_cases))));
    }

    // Default case.
    let default = opt(s, default_case_stmt)?;

    // Close cases bracket.
    recovering_token(s, TokenKind::Close(Delim::Brace));

    Ok(SwitchStmt {
        span: s.span(lo),
        target: controlling_expr,
        cases,
        default,
    })
}

/// Grammar: `CASE expressionList scope`.
fn case_stmt(s: &mut ParserContext) -> Result<SwitchCase> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Case))?;

    let controlling_label = expr::expr_list(s)?;
    if controlling_label.is_empty() {
        s.push_error(Error::new(ErrorKind::MissingSwitchCaseLabels(s.span(lo))));
    }

    let block = parse_block(s)?;

    Ok(SwitchCase {
        span: s.span(lo),
        labels: list_from_iter(controlling_label),
        block,
    })
}

/// Grammar: `DEFAULT scope`.
fn default_case_stmt(s: &mut ParserContext) -> Result<Block> {
    token(s, TokenKind::Keyword(Keyword::Default))?;
    parse_block(s)
}

/// Grammar: `statement | scope`.
fn parse_block_or_stmt(s: &mut ParserContext) -> Result<Stmt> {
    if let Some(block) = opt(s, parse_block)? {
        Ok(Stmt {
            span: block.span,
            annotations: Default::default(),
            kind: Box::new(StmtKind::Block(block)),
        })
    } else {
        parse(s)
    }
}

/// Grammar `IF LPAREN expression RPAREN if_body=statementOrScope (ELSE else_body=statementOrScope)?`.
/// Source: <https://openqasm.com/language/classical.html#if-else-statements>.
pub fn parse_if_stmt(s: &mut ParserContext) -> Result<IfStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::If))?;
    token(s, TokenKind::Open(Delim::Paren))?;
    let condition = expr::expr(s)?;
    recovering_token(s, TokenKind::Close(Delim::Paren));

    let if_block = parse_block_or_stmt(s)?;
    let else_block = if opt(s, |s| token(s, TokenKind::Keyword(Keyword::Else)))?.is_some() {
        Some(parse_block_or_stmt(s)?)
    } else {
        None
    };

    Ok(IfStmt {
        span: s.span(lo),
        condition,
        if_body: if_block,
        else_body: else_block,
    })
}

/// Ranges in for loops are a bit different. They must have explicit start and end.
/// Grammar `LBRACKET start=expression COLON (step=expression COLON)? stop=expression]`.
/// Reference: <https://openqasm.com/language/classical.html#for-loops>.
fn for_loop_range_expr(s: &mut ParserContext) -> Result<RangeDefinition> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Open(Delim::Bracket))?;
    let start = Some(expr::expr(s)?);
    token(s, TokenKind::Colon)?;

    // QASM ranges have the pattern [start : (step :)? end]
    // We assume the second expr is the `end`.
    let mut end = Some(expr::expr(s)?);
    let mut step = None;

    // If we find a third expr, then the second expr was the `step`.
    // and this third expr is the actual `end`.
    if token(s, TokenKind::Colon).is_ok() {
        step = end;
        end = Some(expr::expr(s)?);
    }

    recovering_token(s, TokenKind::Close(Delim::Bracket));

    Ok(RangeDefinition {
        span: s.span(lo),
        start,
        end,
        step,
    })
}

/// Parses the `(setExpression | LBRACKET rangeExpression RBRACKET | expression)`
/// part of a for loop statement.
/// Reference: <https://openqasm.com/language/classical.html#for-loops>.
fn for_loop_iterable_expr(s: &mut ParserContext) -> Result<EnumerableSet> {
    if let Some(range) = opt(s, for_loop_range_expr)? {
        Ok(EnumerableSet::RangeDefinition(range))
    } else if let Some(set) = opt(s, expr::set_expr)? {
        Ok(EnumerableSet::DiscreteSet(set))
    } else {
        Ok(EnumerableSet::Expr(expr::expr(s)?))
    }
}

/// Grammar:
/// `FOR scalarType Identifier IN (setExpression | LBRACKET rangeExpression RBRACKET | expression) body=statementOrScope`.
/// Reference: <https://openqasm.com/language/classical.html#for-loops>.
pub fn parse_for_stmt(s: &mut ParserContext) -> Result<ForStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::For))?;
    let ty = scalar_type(s)?;
    let ident = prim::ident(s)?;
    token(s, TokenKind::Keyword(Keyword::In))?;
    let set_declaration = Box::new(for_loop_iterable_expr(s)?);
    let block = parse_block_or_stmt(s)?;

    Ok(ForStmt {
        span: s.span(lo),
        ty,
        ident,
        set_declaration,
        body: block,
    })
}

/// Grammar: `WHILE LPAREN expression RPAREN body=statementOrScope`.
/// Reference: <https://openqasm.com/language/classical.html#while-loops>.
pub fn parse_while_loop(s: &mut ParserContext) -> Result<WhileLoop> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::While))?;
    token(s, TokenKind::Open(Delim::Paren))?;
    let while_condition = expr::expr(s)?;
    recovering_token(s, TokenKind::Close(Delim::Paren));
    let block = parse_block_or_stmt(s)?;

    Ok(WhileLoop {
        span: s.span(lo),
        while_condition,
        body: block,
    })
}

/// Grammar: `CONTINUE SEMICOLON`.
fn parse_continue_stmt(s: &mut ParserContext) -> Result<ContinueStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Continue))?;
    recovering_semi(s);
    Ok(ContinueStmt { span: s.span(lo) })
}

/// Grammar: `BREAK SEMICOLON`.
fn parse_break_stmt(s: &mut ParserContext) -> Result<BreakStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Break))?;
    recovering_semi(s);
    Ok(BreakStmt { span: s.span(lo) })
}

/// Grammar: `END SEMICOLON`.
fn parse_end_stmt(s: &mut ParserContext) -> Result<EndStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::End))?;
    recovering_semi(s);
    Ok(EndStmt { span: s.span(lo) })
}

/// Grammar: `expression SEMICOLON`.
fn parse_expression_stmt(s: &mut ParserContext, lhs: Option<Expr>) -> Result<ExprStmt> {
    let expr = if let Some(lhs) = lhs {
        expr::expr_with_lhs(s, lhs)?
    } else {
        expr::expr(s)?
    };
    recovering_semi(s);
    Ok(ExprStmt {
        span: s.span(expr.span.lo),
        expr,
    })
}

/// Grammar: `LET Identifier EQUALS aliasExpression SEMICOLON`.
fn parse_alias_stmt(s: &mut ParserContext) -> Result<AliasDeclStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Let))?;
    let ident = Identifier::Ident(Box::new(prim::ident(s)?));
    token(s, TokenKind::Eq)?;
    let exprs = expr::alias_expr(s)?;
    recovering_semi(s);

    Ok(AliasDeclStmt {
        ident,
        exprs,
        span: s.span(lo),
    })
}

/// Grammar: `BOX designator? scope`.
fn parse_box(s: &mut ParserContext) -> Result<BoxStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Box))?;
    let duration = opt(s, designator)?;
    let body = parse_box_body(s)?;

    Ok(BoxStmt {
        span: s.span(lo),
        duration,
        body,
    })
}

fn parse_box_body(s: &mut ParserContext) -> Result<List<Stmt>> {
    token(s, TokenKind::Open(Delim::Brace))?;
    let stmts = barrier(
        s,
        &[TokenKind::Close(Delim::Brace)],
        parse_many_boxable_stmt,
    )?;
    recovering_token(s, TokenKind::Close(Delim::Brace));
    Ok(stmts)
}

fn parse_many_boxable_stmt(s: &mut ParserContext) -> Result<List<Stmt>> {
    let stmts = many(s, |s| {
        recovering(
            s,
            |span| Stmt {
                span,
                kind: Box::new(StmtKind::Err),
                annotations: Box::new([]),
            },
            &[TokenKind::Semicolon],
            parse,
        )
    });

    Ok(list_from_iter(stmts?))
}

/// In QASM3, it is hard to disambiguate between a quantum-gate-call missing modifiers
/// and expression statements. Consider the following expressions:
///  1. `Ident(2, 3) a;`
///  2. `Ident(2, 3) + a * b;`
///  3. `Ident(2, 3);`
///  4. `Ident(2, 3)[1];`
///  5. `Ident;`
///  6. `Ident[4us] q;`
///  7. `Ident[4];`
///  8. `Ident q;`
///
/// (1) is a quantum-gate-call, (2) is a binary operation, (3) is a function call, and
/// (4) is an identifer. We don't know for sure until we see the what is beyond the gate
/// name and its potential classical parameters.
///
/// Therefore, we parse the gate name and its potential parameters using the expr parser.
/// If the expr is a function call or an identifier and it is followed by qubit arguments,
/// we reinterpret the expression as a quantum gate.
///
/// Grammar:
///  `gateModifier* Identifier (LPAREN expressionList? RPAREN)? designator? gateOperandList  SEMICOLON
/// | gateModifier* GPHASE     (LPAREN expressionList? RPAREN)? designator? gateOperandList? SEMICOLON`.
fn parse_gate_call_stmt(s: &mut ParserContext) -> Result<StmtKind> {
    let lo = s.peek().span.lo;
    let modifiers = list_from_iter(many(s, gate_modifier)?);

    // If the next token is `gphase`, parse a gphase instead, which has optional operands.
    if s.peek().kind == TokenKind::GPhase {
        let gphase = parse_gphase(s, lo, modifiers)?;
        return Ok(StmtKind::GPhase(gphase));
    }

    // 1. ident = ...
    // 2. parameters? = ... Option<List>
    // 3. designator? = ...
    // 4. qubits = ... -> qubits.is_empty()

    // cases: (no qubits)
    //   ident + parameters -> function call
    //   ident + designator -> indexed ident

    // As explained in the docstring, we parse the gate using the expr parser.
    let gate_or_expr = expr::expr(s)?;

    let mut duration = opt(s, designator)?;
    let qubits = gate_operand_list(s)?;

    // If didn't parse modifiers, a duration, nor qubit args then this is an expr, not a gate call.
    if modifiers.is_empty() && duration.is_none() && qubits.is_empty() {
        return Ok(StmtKind::ExprStmt(parse_expression_stmt(
            s,
            Some(gate_or_expr),
        )?));
    }

    // We parse the recovering semi after we call parse_expr_stmt.
    recovering_semi(s);

    // Reinterpret the function call or ident as a gate call.
    let (name, args) = match *gate_or_expr.kind {
        ExprKind::FunctionCall(FunctionCall { name, args, .. }) => (name, args),
        ExprKind::Ident(ident) => (ident, Default::default()),
        ExprKind::IndexExpr(index_expr) => reinterpret_index_expr(index_expr, &mut duration)?,
        _ => {
            return Err(Error::new(ErrorKind::ExpectedItem(
                TokenKind::Identifier,
                gate_or_expr.span,
            )))
        }
    };

    if qubits.is_empty() {
        s.push_error(Error::new(ErrorKind::MissingGateCallOperands(s.span(lo))));
    }

    Ok(StmtKind::GateCall(GateCall {
        span: s.span(lo),
        modifiers,
        name,
        args,
        qubits,
        duration,
    }))
}

/// This parser is used to disambiguate statements starting with an index
/// identifier. It is a simplified version of `parse_gate_call_stmt`.
fn parse_gate_call_with_expr(s: &mut ParserContext, gate_or_expr: Expr) -> Result<StmtKind> {
    let lo = gate_or_expr.span.lo;
    let mut duration = opt(s, designator)?;
    let qubits = gate_operand_list(s)?;

    // If didn't parse modifiers, a duration, nor qubit args then this is an expr, not a gate call.
    if duration.is_none() && qubits.is_empty() {
        return Ok(StmtKind::ExprStmt(parse_expression_stmt(
            s,
            Some(gate_or_expr),
        )?));
    }

    // We parse the recovering semi after we call parse_expr_stmt.
    recovering_semi(s);

    // Reinterpret the function call or ident as a gate call.
    let (name, args) = match *gate_or_expr.kind {
        ExprKind::FunctionCall(FunctionCall { name, args, .. }) => (name, args),
        ExprKind::Ident(ident) => (ident, Default::default()),
        ExprKind::IndexExpr(index_expr) => reinterpret_index_expr(index_expr, &mut duration)?,
        _ => {
            return Err(Error::new(ErrorKind::ExpectedItem(
                TokenKind::Identifier,
                gate_or_expr.span,
            )))
        }
    };

    if qubits.is_empty() {
        s.push_error(Error::new(ErrorKind::MissingGateCallOperands(s.span(lo))));
    }

    Ok(StmtKind::GateCall(GateCall {
        span: s.span(lo),
        modifiers: Default::default(),
        name,
        args,
        qubits,
        duration,
    }))
}

/// This helper function reinterprets an indexed expression as
/// a gate call. There are two valid cases we are interested in:
///  1. Ident[4]
///  2. Ident(2, 3)[4]
///
/// Case (1) is an indexed identifier, in which case we want to
/// reinterpret it as a gate followed by a designator.
/// Case (2) is an indexed function call, in which case we want to
/// reinterpret it as a parametrized gate followed by a designator.
fn reinterpret_index_expr(
    index_expr: IndexExpr,
    duration: &mut Option<Expr>,
) -> Result<(Ident, List<Expr>)> {
    let IndexExpr {
        collection, index, ..
    } = index_expr;

    if let IndexElement::IndexSet(set) = index {
        if set.values.len() == 1 {
            let first_elt: IndexSetItem = (*set.values[0]).clone();
            if let IndexSetItem::Expr(expr) = first_elt {
                if duration.is_none() {
                    match *collection.kind {
                        ExprKind::Ident(name) => {
                            *duration = Some(expr);
                            return Ok((name, Default::default()));
                        }
                        ExprKind::FunctionCall(FunctionCall { name, args, .. }) => {
                            *duration = Some(expr);
                            return Ok((name, args));
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    Err(Error::new(ErrorKind::InvalidGateCallDesignator(
        index_expr.span,
    )))
}

/// Grammar:
/// `gateModifier* GPHASE (LPAREN expressionList? RPAREN)? designator? gateOperandList? SEMICOLON`
fn parse_gphase(
    s: &mut ParserContext,
    lo: u32,
    modifiers: List<QuantumGateModifier>,
) -> Result<GPhase> {
    let gphase_token_lo = s.peek().span.lo;
    token(s, TokenKind::GPhase)?;
    let gphase_token_span = s.span(gphase_token_lo);

    let args_lo = s.peek().span.lo;
    let args = opt(s, |s| {
        token(s, TokenKind::Open(Delim::Paren))?;
        let exprs = expr::expr_list(s)?;
        recovering_token(s, TokenKind::Close(Delim::Paren));
        Ok(list_from_iter(exprs))
    })?
    .unwrap_or_default();

    if args.len() != 1 {
        s.push_error(Error::new(ErrorKind::GPhaseInvalidArguments(
            s.span(args_lo),
        )));
    }

    let duration = opt(s, designator)?;
    let qubits = gate_operand_list(s)?;
    recovering_semi(s);

    Ok(GPhase {
        span: s.span(lo),
        gphase_token_span,
        modifiers,
        args,
        qubits,
        duration,
    })
}

/// Grammar:
/// `(
///     INV
///     | POW LPAREN expression RPAREN
///     | (CTRL | NEGCTRL) (LPAREN expression RPAREN)?
/// ) AT`.
fn gate_modifier(s: &mut ParserContext) -> Result<QuantumGateModifier> {
    let lo = s.peek().span.lo;

    let kind = if opt(s, |s| token(s, TokenKind::Inv))?.is_some() {
        GateModifierKind::Inv
    } else if opt(s, |s| token(s, TokenKind::Pow))?.is_some() {
        token(s, TokenKind::Open(Delim::Paren))?;
        let expr = expr::expr(s)?;
        recovering_token(s, TokenKind::Close(Delim::Paren));
        GateModifierKind::Pow(expr)
    } else if opt(s, |s| token(s, TokenKind::Ctrl))?.is_some() {
        let expr = opt(s, |s| {
            token(s, TokenKind::Open(Delim::Paren))?;
            let expr = expr::expr(s)?;
            recovering_token(s, TokenKind::Close(Delim::Paren));
            Ok(expr)
        })?;
        GateModifierKind::Ctrl(expr)
    } else {
        token(s, TokenKind::NegCtrl)?;
        let expr = opt(s, |s| {
            token(s, TokenKind::Open(Delim::Paren))?;
            let expr = expr::expr(s)?;
            recovering_token(s, TokenKind::Close(Delim::Paren));
            Ok(expr)
        })?;
        GateModifierKind::NegCtrl(expr)
    };

    recovering_token(s, TokenKind::At);

    Ok(QuantumGateModifier {
        span: s.span(lo),
        kind,
    })
}

/// Grammar: `gateOperand (COMMA gateOperand)* COMMA?`.
fn gate_operand_list(s: &mut ParserContext) -> Result<List<GateOperand>> {
    Ok(list_from_iter(seq(s, gate_operand)?.0))
}

/// Grammar: `DEFCALGRAMMAR StringLiteral SEMICOLON`.
fn parse_calibration_grammar_stmt(s: &mut ParserContext) -> Result<CalibrationGrammarStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::DefCalGrammar))?;

    let next = s.peek();
    let lit = expr::lit(s)?;

    recovering_semi(s);
    if let Some(lit) = lit {
        if let LiteralKind::String(name) = lit.kind {
            return Ok(CalibrationGrammarStmt {
                span: s.span(lo),
                name: name.to_string(),
            });
        }
    };

    Err(Error::new(ErrorKind::Rule(
        "string literal",
        next.kind,
        next.span,
    )))
}

/// We don't support `defcal` block statements in the compiler. Therefore
/// the parser just goes through the tokens in a defcal block and ignores them.
/// Grammar: `DEFCAL pushmode(eatUntilOpenBrace) pushmode(eatUntilBalancedClosingBrace)`.
fn parse_defcal_stmt(s: &mut ParserContext) -> Result<DefCalStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::DefCal))?;

    // Once we have parsed the `defcal` token, we eat all the tokens until we see an open brace.
    while !matches!(
        s.peek().kind,
        TokenKind::Open(Delim::Brace) | TokenKind::Eof
    ) {
        s.advance();
    }

    token(s, TokenKind::Open(Delim::Brace))?;
    let mut level: u32 = 1;

    loop {
        match s.peek().kind {
            TokenKind::Eof => {
                s.advance();
                return Err(Error::new(ErrorKind::Token(
                    TokenKind::Close(Delim::Brace),
                    TokenKind::Eof,
                    s.span(lo),
                )));
            }
            TokenKind::Open(Delim::Brace) => {
                s.advance();
                level += 1;
            }
            TokenKind::Close(Delim::Brace) => {
                s.advance();
                level -= 1;
                if level == 0 {
                    return Ok(DefCalStmt { span: s.span(lo) });
                }
            }
            _ => s.advance(),
        }
    }
}

/// We don't support `cal` block statements in the compiler. Therefore
/// the parser just goes through the tokens in a cal block and ignores them.
/// Grammar: `CAL OPEN_BRACE pushmode(eatUntilBalancedClosingBrace)`.
fn parse_cal(s: &mut ParserContext) -> Result<CalibrationStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Cal))?;
    token(s, TokenKind::Open(Delim::Brace))?;
    let mut level: u32 = 1;

    loop {
        match s.peek().kind {
            TokenKind::Eof => {
                s.advance();
                return Err(Error::new(ErrorKind::Token(
                    TokenKind::Close(Delim::Brace),
                    TokenKind::Eof,
                    s.span(lo),
                )));
            }
            TokenKind::Open(Delim::Brace) => {
                s.advance();
                level += 1;
            }
            TokenKind::Close(Delim::Brace) => {
                s.advance();
                level -= 1;
                if level == 0 {
                    return Ok(CalibrationStmt { span: s.span(lo) });
                }
            }
            _ => s.advance(),
        }
    }
}

/// Grammar: `BARRIER gateOperandList? SEMICOLON`.
fn parse_barrier(s: &mut ParserContext) -> Result<BarrierStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Barrier))?;
    let qubits = gate_operand_list(s)?;
    recovering_semi(s);

    Ok(BarrierStmt {
        span: s.span(lo),
        qubits,
    })
}

/// Grammar: `DELAY designator gateOperandList? SEMICOLON`.
fn parse_delay(s: &mut ParserContext) -> Result<DelayStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Delay))?;
    let duration = designator(s)?;
    let qubits = gate_operand_list(s)?;
    recovering_semi(s);

    Ok(DelayStmt {
        span: s.span(lo),
        duration,
        qubits,
    })
}

/// Grammar: `RESET gateOperand SEMICOLON`.
fn parse_reset(s: &mut ParserContext) -> Result<ResetStmt> {
    let lo = s.peek().span.lo;
    token(s, TokenKind::Keyword(Keyword::Reset))?;
    let reset_token_span = s.span(lo);
    let operand = Box::new(gate_operand(s)?);
    recovering_semi(s);

    Ok(ResetStmt {
        span: s.span(lo),
        reset_token_span,
        operand,
    })
}

/// Grammar: `measureExpression (ARROW indexedIdentifier)? SEMICOLON`.
fn parse_measure_stmt(s: &mut ParserContext) -> Result<MeasureArrowStmt> {
    let lo = s.peek().span.lo;
    let measure = expr::measure_expr(s)?;

    let target = opt(s, |s| {
        token(s, TokenKind::Arrow)?;
        Ok(Box::new(indexed_identifier(s)?))
    })?;

    recovering_semi(s);

    Ok(MeasureArrowStmt {
        span: s.span(lo),
        measurement: measure,
        target,
    })
}
