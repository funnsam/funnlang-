use super::super::*;

// macro_rules! wrap_result {
// ($res: expr, $fail: expr) => {
//         match $res {
//             Ok(a) => Ok(a),
//             Err(_) => Err($fail),
//         }
//     };
// }

macro_rules! wrap_option {
    ($res: expr, $fail: expr) => {
        match $res {
            Some(a) => Ok(a),
            None => Err($fail),
        }
    };
}


pub fn parse(buf: &mut Buffer<AToken>, src: &str) -> Result<AST, Vec<ACompileError>> {
    let mut _ast = AST::new();
    let mut ast = vec![(&mut _ast, Span::default())];

    let mut comp_errs: Vec<ACompileError> = Vec::new();

    macro_rules! ast {
        () => { ast.last_mut().unwrap() };
    }

    macro_rules! ast_push {
        ($val: expr, $span: expr) => {
            ast!().0.push(($val, $span.clone()));
            ast!().1.end = $span.end;
        }
    }

    'main_loop: while let Some((tok, span)) = buf.next().cloned() {
        macro_rules! consider_error {
            ($expr: expr) => {
                match $expr {
                    Err((e, s)) => {
                        comp_errs.push((Box::new(e), s));

                        buf.rewind();
                        while let Some((t, _)) = buf.next() {
                            if matches!(t, Token::Semicolon | Token::CuBracketS) {
                                break;
                            }
                        }
                        continue 'main_loop;
                    },
                    Ok(a) => a,
                }
            };
        }

        macro_rules! error {
            ($r: expr, $s: expr) => {{
                comp_errs.push((Box::new($r), $s));
                while let Some((t, _)) = buf.next() {
                    if matches!(t, Token::Semicolon | Token::CuBracketS) {
                        break;
                    }
                }
                continue 'main_loop;
            }};
        }

        macro_rules! unwrap_ident {
            ($span: expr) => {
                match buf.next() {
                    Some((Token::Ident, s)) => (src[s.start..s.end].to_string(), s.clone()),
                    Some((_, s)) => error!(ParseError::UnexpectedToken, s.clone()),
                    None => error!(ParseError::RanOutTokens, $span),
                }
            };
        }

        macro_rules! assert_token {
            ($intended: ident) => {
                match buf.next() {
                    Some((Token::$intended, _)) => (),
                    Some((_, s)) => error!(ParseError::UnexpectedToken, s.clone()),
                    None => error!(ParseError::RanOutTokens, buf.prev().unwrap().1.clone()),
                }
            };
        }

        match tok {
            Token::Semicolon => {},
            Token::Var => {
                let (ident, idspan) = unwrap_ident!(span);
                let (expr, end) = match buf.next() {
                    Some((Token::Operator(Operator::Assign), _)) => {
                        let expr = consider_error!(parse_expr(buf, src));
                        (Some(expr), buf.current().unwrap().1.start)
                    },
                    Some((Token::Semicolon, _)) => (None, buf.current().unwrap().1.start),
                    Some((_, s)) => error!(ParseError::UnexpectedToken, s.clone()),
                    None => error!(ParseError::RanOutTokens, span),
                };
                let s = Span { start: span.start, end };
                ast_push!(Node::VarDeclare { ident: (ident, idspan), expr }, s);
            },
            Token::Return => {
                let (expr, end) = match buf.next() {
                    Some((Token::Semicolon, _)) => (None, buf.current().unwrap().1.start),
                    Some((_, _)) => {
                        let expr = consider_error!(parse_expr(buf, src));
                        (Some(expr), buf.current().unwrap().1.start)
                    },
                    None => error!(ParseError::RanOutTokens, span),
                };
                let span = Span { start: span.start, end };
                ast_push!(Node::Return(expr), span);
            },
            Token::Func => {
                let (ident, idspan) = unwrap_ident!(span);
                assert_token!(RoBracketS);

                let mut params = Vec::new();
                let mut en = 0;
                while let Some((tok, sp)) = buf.next().cloned() {
                    match tok {
                        Token::RoBracketE => {
                            en = sp.end;
                            break
                        },
                        Token::Ident => {
                            let id = &src[sp.start..sp.end];
                            let typ = consider_error!(parse_type(buf, src));
                            let end = typ.1.end;
                            params.push(((id.to_string(), sp.clone()), typ, Span { start: sp.start, end }));

                            match buf.peek() {
                                Some((Token::Comma, _)) => buf.idx += 1,
                                None => error!(ParseError::RanOutTokens, buf.prev().unwrap().1.clone()),
                                _ => ()
                            }
                        },
                        _ => error!(ParseError::UnexpectedToken, sp.clone()),
                    }
                }

                assert_token!(CuBracketS);
                let a = AST::new();
                let mut_a = unsafe { &mut *(&a as *const AST as *mut AST) };
                let span = Span { start: span.start, end: en };
                ast_push!(Node::FunctionDeclare { ident: (ident, idspan), params, body: a }, span);
                ast.push((mut_a, span));

            },
            Token::CuBracketE => {
                if ast.len() == 1 {
                    error!(ParseError::UnstartedBracket, span);
                } else {
                    ast!().1.end = span.end;
                    ast.pop();
                }
            },
            _ => {
                buf.rewind();
                let expr = consider_error!(parse_expr(buf, src));
                let span = expr.1.clone();
                ast_push!(Node::Expr(expr), Span { start: span.start, end: buf.current().unwrap().1.start });
            },
        }
    }

    if ast.len() != 1 {
        comp_errs.push((Box::new(ParseError::UnendedBracket), ast!().1.clone()));
    }

    if comp_errs.len() == 0 {
        Ok(_ast)
    } else {
        Err(comp_errs)
    }
}

fn parse_type(buf: &mut Buffer<AToken>, src: &str) -> Result<AType, (ParseError, Span)> {
    let mut typ_opers: Vec<(TypeOperators, Span)> = Vec::new();

    macro_rules! assert_token {
        ($intended: ident) => {
            match buf.next() {
                Some((Token::$intended, _)) => (),
                Some((_, s)) => return Err((ParseError::UnexpectedToken, s.clone())),
                None => return Err((ParseError::RanOutTokens, buf.prev().unwrap().1.clone())),
            }
        };
    }

    while let Some((tok, span)) = buf.next().cloned() {
        match tok {
            Token::Ident => {
                let mut typ = (Type::from_str(&src[span.start..span.end]), span);
                for (op, sp) in typ_opers.iter().rev() {
                    match op {
                        TypeOperators::Pointer => typ = (Type::Pointer(Box::new(typ)), sp.clone()),
                        TypeOperators::Slice => typ = (Type::Slice(Box::new(typ)), sp.clone()),
                        TypeOperators::Array(s) => typ = (Type::Array(Box::new(typ), *s), sp.clone()),
                    }
                }
                return Ok(typ)
            },
            Token::Operator(Operator::BAnd) => typ_opers.push((TypeOperators::Pointer, span)),
            Token::Operator(Operator::LAnd) => {
                typ_opers.push((TypeOperators::Pointer, Span { start: span.start, end: span.start+1 }));
                typ_opers.push((TypeOperators::Pointer, Span { start: span.start+1, end: span.end }));
            },
            Token::SqBracketS => {
                match buf.next() {
                    Some((Token::SqBracketE, sp)) => {
                        typ_opers.push((TypeOperators::Slice, Span { start: span.start, end: sp.end }));
                    },
                    Some((Token::Integer(i), sp)) => {
                        typ_opers.push((TypeOperators::Array(*i), Span { start: span.start, end: sp.end }));
                        assert_token!(SqBracketE);
                    },
                    Some((_, sp)) => return Err((ParseError::UnexpectedToken, sp.clone())),
                    None => return Err((ParseError::RanOutTokens, buf.prev().unwrap().1.clone())),
                }
            },
            _ => return Err((ParseError::UnexpectedToken, span)),
        }
    }

    Err((ParseError::RanOutTokens, buf.prev().unwrap().1.clone()))
}

fn parse_expr(buf: &mut Buffer<AToken>, src: &str) -> Result<AExpr, (ParseError, Span)> {
    let mut out = Vec::new();
    let mut ops: Vec<(Operator, Span, bool)> = Vec::new();
    let mut last = Token::None;
    let mut fn_args = Vec::new();

    macro_rules! pop_oper_to_out {
        ($op: expr, $span: expr, $unary:expr) => {
            match &$op {
                Operator::FnCall(id) => {
                    let n = fn_args.pop().unwrap();
                    let mut op = Vec::with_capacity(n);
                    let mut span = $span.clone();

                    for _ in 0..n {
                        let (o, s) = wrap_option!(out.pop(), (ParseError::RanOutOperands, $span.clone()))?;
                        span.start = span.start.min(s.start);
                        span.end = span.end.max(s.end);
                        op.push((o, s));
                    }


                    op.reverse();
                    out.push((Expr::FnCall { id: Box::new(id.clone()), op }, span));
                    ops.pop();
                },
                Operator::RoBracketS => {
                    ops.pop();
                },
                Operator::Index(id) => {
                    let idx = wrap_option!(out.pop(), (ParseError::RanOutOperands, $span.clone()))?;
                    let s = $span.start.min(idx.1.start);
                    let e = $span.end.max(idx.1.end);
                    let span = Span { start: s, end: e };
                    out.push((Expr::Index { lhs: Box::new(id.clone()), rhs: Box::new(idx) }, span));
                    ops.pop();
                },
                _ => { pop_oper_to_out_no_fn!($op, $span, $unary); },
            }
        };
    }

    macro_rules! pop_oper_to_out_no_fn {
        ($op: expr, $span: expr, $unary:expr) => {
            if !$unary {
                let rhs = wrap_option!(out.pop(), (ParseError::RanOutOperands, $span.clone()))?;
                let lhs = wrap_option!(out.pop(), (ParseError::RanOutOperands, $span.clone()))?;
                let s = $span.start.min(lhs.1.start).min(rhs.1.start);
                let e = $span.end.max(lhs.1.end).max(rhs.1.end);
                let span = Span { start: s, end: e };
                out.push((Expr::BiOp { lhs: Box::new(lhs), rhs: Box::new(rhs), op: Box::new($op.clone()) }, span));
            } else {
                let opr = wrap_option!(out.pop(), (ParseError::RanOutOperands, $span.clone()))?;
                let s = $span.start.min(opr.1.start);
                let e = $span.end.max(opr.1.end);
                let span = Span { start: s, end: e };
                out.push((Expr::UnOp { opr: Box::new(opr), op: Box::new($op.clone()) }, span));
            }
            ops.pop();
        };
    }

    macro_rules! infers {
        () => {
            matches!(last, Token::RoBracketE | Token::SqBracketE | Token::Ident)
        };
    }

    let mut expr_span = Span { start: usize::MAX, end: usize::MIN };

    while let Some((tok, span)) = buf.next() {
        let tok = tok.clone();
        let span = span.clone();

        match &tok {
            Token::Semicolon => {
                break
            },
            Token::Integer(i) => {
                if matches!(last, Token::Integer(_) | Token::Ident | Token::RoBracketE | Token::SqBracketE) {
                    return Err((ParseError::UnexpectedToken, span));
                }
                out.push((Expr::Integer(*i as i128), span.clone()));
            },
            Token::Ident => {
                if matches!(last, Token::Integer(_) | Token::Ident | Token::RoBracketE | Token::SqBracketE) {
                    return Err((ParseError::UnexpectedToken, span));
                } else {
                    out.push((Expr::Ident(src[span.start..span.end].to_string()), span.clone()))
                }
            },
            Token::Operator(o1) => {
                let un = matches!(last, Token::Operator(_) | Token::None);
                while let Some((o2, span, unary)) = ops.last() {
                    if matches!(o2, Operator::RoBracketS | Operator::FnCall(_) | Operator::Index(_)) || !(o2.percedence(*unary) > o1.percedence(un) || (o1.percedence(un) == o2.percedence(*unary) && o1.is_left())) {
                        break;
                    }

                    pop_oper_to_out_no_fn!(o2, span, unary);
                }
                ops.push((o1.clone(), span.clone(), un));
            },
            Token::Comma => {
                while let Some((op, span, unary)) = ops.last() {
                    if matches!(op, Operator::FnCall(_)) {
                        break;
                    }

                    pop_oper_to_out_no_fn!(op, span, unary);
                }
                *wrap_option!(fn_args.last_mut(), (ParseError::UnexpectedToken, span.clone()))? += 1;
            },
            Token::RoBracketS => {
                if matches!(last, Token::Integer(_)) {
                    return Err((ParseError::UnexpectedToken, span));
                } else if infers!() { // fn calls
                    let func = out.pop().unwrap();
                    let start = func.1.start;
                    ops.push((Operator::FnCall(func), Span { start, end: span.end }, false));
                    fn_args.push(if matches!(buf.peek().unwrap_or(&(Token::None, Span::default())).0, Token::RoBracketE) { 0 } else { 1 });
                } else {
                    ops.push((Operator::RoBracketS, span.clone(), false));
                }
            },
            Token::RoBracketE => {
                let mut errs = true;
                while let Some((op, span, unary)) = ops.last().cloned() {
                    pop_oper_to_out!(op, span, unary);
                    if matches!(op, Operator::FnCall(_) | Operator::RoBracketS) {
                        errs = false;
                        break;
                    } else if matches!(op, Operator::Index(_)) {
                    return Err((ParseError::BracketNotMatch, span));
                    }
                }

                if errs {
                    return Err((ParseError::UnstartedBracket, span))
                }
            },
            Token::SqBracketS => {
                if infers!() {
                    let id = out.pop().unwrap();
                    let start = id.1.start;
                    ops.push((Operator::Index(id), Span { start, end: span.end }, false));
                } else {
                    return Err((ParseError::UnexpectedToken, span))
                }
            },
            Token::SqBracketE => {
                let mut errs = true;
                while let Some((op, span, unary)) = ops.last().cloned() {
                    pop_oper_to_out!(op, span, unary);
                    if matches!(op, Operator::Index(_)) {
                        errs = false;
                        break;
                    } else if matches!(op, Operator::FnCall(_) | Operator::RoBracketS) {
                        return Err((ParseError::BracketNotMatch, span))
                    }
                }

                if errs {
                    return Err((ParseError::UnstartedBracket, span))
                }
            },
            _ => return Err((ParseError::UnexpectedToken, span)),
        }

        expr_span.start = expr_span.start.min(span.start);
        expr_span.end = expr_span.end.max(span.end);
        last = buf.current().unwrap().0.clone();
    }


    while let Some((op, span, unary)) = ops.last().cloned() {
        if matches!(op, Operator::RoBracketS | Operator::Index(_)) {
            return Err((ParseError::UnendedBracket, span.clone()));
        } else if matches!(op, Operator::FnCall(_)) {
            return Err((ParseError::UnendedFnCall, span.clone()));
        } 

        pop_oper_to_out!(op, span, unary);
    }

    if out.len() == 1 {
        Ok(out[0].clone())
    } else {
        Err((ParseError::ExprParseError, expr_span))
    }
}
