use crate::types::Expr;
use crate::lex::lex;
use crate::parse::parse;
use crate::eval::{eval, Environment, EvalResult};

/// Lexes, parses, and evaluates the given program.
pub fn run_interpreter(program: &str) -> EvalResult {
    match lex(&program){
        Err(e) => EvalResult::Err(format!("Lex error: {:?}", e)),
        Ok(tokens) => match parse(&tokens) {
            Err(e) => EvalResult::Err(format!("Parse error: {:?}", e)),
            Ok(expr) => {
                let mut env = Environment::default();
                match eval(expr.clone(), &mut env) {
                    EvalResult::Err(e) => EvalResult::Err(e),
                    EvalResult::Expr(expr) => match &*expr.clone() {
                        Expr::Symbol(s) => EvalResult::Expr(Expr::symbol(&s)),
                        Expr::FNum(n) => EvalResult::Expr(Expr::fnum(*n)),
                        Expr::List(l) => EvalResult::Expr(Expr::list(&l)),
                    } ,
                    EvalResult::Unit => EvalResult::Unit ,
                }
            },
        },
    }
}
