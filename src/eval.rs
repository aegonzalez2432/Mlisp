use crate::types::Expr;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub enum EvalResult {
    Err(String),
    Expr(Rc<Expr>),
    Unit,
}

#[derive(Debug)]
pub struct Environment {
    pub contexts: Vec<HashMap<String, (Vec<String>, Rc<Expr>)>>,
}

impl Environment {
    pub fn empty() -> Environment {
        Environment {
            contexts: Vec::new(),
        }
    }

    /// Helper function for tests
    pub fn from_vars(vars: &[(&str, Rc<Expr>)]) -> Environment {
        let mut env = Environment::empty();
        env.push_context();
        vars.iter().for_each(|(name, expr)| {
            let _ = env.add_var(name, expr.clone());
        });
        env
    }

    pub fn default() -> Environment {
        let defaults: HashMap<String, (Vec<String>, Rc<Expr>)> = [
            ("False".into(), (Vec::new(), Expr::list(&[]))), ("True".into(), (Vec::new(), Expr::list(&[Expr::fnum(1.0)]))),
        ].iter().cloned().collect();
        Environment{
            contexts: vec![defaults],
        }


    }

    /// Looks up the given symbol in the Environment.
    pub fn lookup(&self, symbol: &str) -> Option<(Vec<String>, Rc<Expr>)> {
        self.contexts.iter().rev()
            .find(|cntxt| cntxt.contains_key(symbol))
            .map(|cntxt| cntxt.get(symbol))
            .flatten().cloned()
    }

    /// Checks whether the given symbol exists in the Environment.
    pub fn contains_key(&self, symbol: &str) -> bool {
        self.contexts.iter().rev()
            .find(|cntxt| cntxt.contains_key(symbol))
            .is_some()
    }

    /// Pushes a new context on the `contexts` stack.
    pub fn push_context(&mut self) {
        self.contexts.push(HashMap::new());
    }

    /// Pops the last context from the `contexts` stack.
    pub fn pop_context(&mut self) {
        self.contexts.pop();
    }

    /// Adds a variable definition to the Environment
    pub fn add_var(&mut self, var: &str, val: Rc<Expr>) -> Result<(), String> {
        self.contexts.last_mut()
            .map_or_else(
                || Err("Environment has no context to add to.".into()),
                |cntxt| { cntxt.insert(var.to_string(), (Vec::new(), val.clone())); Ok(()) },
            )
    }

    /// Adds a function definition to the Environment
    pub fn add_fn(&mut self, name: &str, params: &[String], body: Rc<Expr>) -> Result<(), String> {
        self.contexts.last_mut().map_or(
            Err("Environment does not have a context to add to.".into()),
            |cntxt| {
                let param_names: Vec<String> = params.iter().map(|s| s.to_string()).collect();
                cntxt.insert(name.into(), (param_names, body.clone()));
                Ok(())
            },
        )
    }

    pub fn num_contexts(&self) -> usize {
        self.contexts.len()
    }
}

fn eval_symbol(expr: Rc<Expr>, sym: &str, args: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    env.lookup(sym)
        .map_or_else(
            || EvalResult::Expr(expr),
            |(param_names, expression)| {
                if param_names.is_empty(){
                    eval(expression.clone(), env)
                } else {
                    if args.len() != param_names.len() {
                        return EvalResult::Err(format!("provided {} arguments but expected {}", args.len(), param_names.len()));
                    }
                    let mapped_args: Result<Vec<(String, Rc<Expr>)>, String> = args.iter().zip(param_names)
                        .map(|(expr, name)| match eval(expr.clone(), env) {
                           EvalResult::Expr(e) => Ok((name.into(), e.clone())),
                           EvalResult::Err(err) => Err(err),
                           _ => Err("Cannot pass Unit as an argument to a function.".into()),
                        }).collect();

                    env.push_context();
                    let result = mapped_args.map_or_else(
                        |e| EvalResult::Err(e),
                        |argum| {
                            argum.iter().for_each(|(name, expr)| { let _ = env.add_var(name, expr.clone()); 
                            }); 
                            eval(expression.clone(), env)
                        },
                    );
                    env.pop_context();
                    result
                }    
            },
        )
}

/// Generates the output printed to standard out when the user calls print.
pub fn gen_print_output(expr: Rc<Expr>, env: &mut Environment) -> String {
    match &*expr {
        Expr::Symbol(s) => {
            match env.lookup(&s) {
                None => s.into(),
                Some((params, e)) if params.len() == 0 => gen_print_output(e, env),
                _ => format!("<func-object: {}>", s.to_string()),
            }
        }
        Expr::FNum(n) => format!("{}", n),
        Expr::List(vals) => {
            let vals_out: Vec<String> = vals.iter().cloned()
                .map(|x| gen_print_output(x, env)).collect();
            format!("({})", vals_out.join(" "))
                
        }
    }
}

fn add_var_to_env(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    if vals.len() != 2 {
        return EvalResult::Err("Invalid variable definition. Should look like (let someVar someExpr)".into(),

        );
    }
    match( &*vals[0], &vals[1]) {
        (Expr::Symbol(s), e) => match eval(e.clone(), env) {
            EvalResult::Expr(e) => env.add_var(s, e)
                .map_or_else(
                    |s| EvalResult::Err(s), 
                    |_| EvalResult::Unit,
                ),
            EvalResult::Unit => EvalResult::Err("cannot assign Unit to a variable.".into()),
            err => err,
        },
        _ => EvalResult::Err("Second element of variable def must be a symbol and third must be expression.".into(),
        ),
    }
}

fn add_fn_to_env(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    if vals.len() != 3 {
        EvalResult::Err("Function definitions must follow the pattern (fn fn-name (arg1 arg2 arg3 .. argn) <Expr>)".into());
    }
    let fn_name = &*vals[0];
    let p_names = &*vals[1];
    let body = &vals[2];
    match(&*fn_name, p_names, body){
        (Expr::Symbol(fn_name), Expr::List(params), body) => {
            let ps: Result<Vec<String>, String> = params.iter().cloned().map(|e| {
                if let Expr::Symbol(n) = &*e {
                    Ok(n.into())
                } else {
                    Err("Function parameters must be symbols.".into())
                }
            }).collect();
            ps.map_or_else(
                |err| EvalResult::Err(err),
                |xs| env.add_fn(fn_name, xs.as_slice(), body.clone()).map_or_else(
                    |err| EvalResult::Err(err),
                    |_| EvalResult::Unit,
                )
            )
        },
        _ => EvalResult::Err("Function definitions must follow the pattern (fn fn-name (arg1 arg2 arg3 .. argn) <Expr>)".into()),
    }
}

fn add_vals(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    if vals.is_empty() {
        return EvalResult::Err("Must perform addition on at least one number".into());
    }
    let total = vals.iter()
        .map(|e| match eval(e.clone(), env) {
            EvalResult::Expr(exp) => match &*exp {
                Expr::FNum(n) => Ok(*n),
                _ => Err(format!("Can only sum numbers, got {:?}", exp)),
            },
            _ => Err(format!("Failed to eval expr: {:?}", e)),
        }).collect::<Result<Vec<f64>, String>>();
    total.map_or_else(
        |err| EvalResult::Err(err),
        |xs| EvalResult::Expr(Expr::fnum(xs.iter().sum())),
    )
}
fn subtract(vals: &Vec<f64>) -> f64 {
    let mut sub = vals[0];
    vals[1..].iter().for_each(|x| {
        sub -= x;
    });
    sub
}
fn sub_vals(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    if vals.is_empty() {
        return EvalResult::Err("Must perform subtraction on at least one number".into());
    }
    let total = vals.iter()
        .map(|e| match eval(e.clone(), env) {
            EvalResult::Expr(exp) => match &*exp {
                Expr::FNum(n) => Ok(*n),
                _ => Err(format!("Can only subtract numers, got {:?}", exp)),
            },
            _ => Err(format!("Failed to eval expr: {:?}", e)),
        }).collect::<Result<Vec<f64>, String>>();
    let firstElement = true;
    total.map_or_else(
        |err| EvalResult::Err(err),
        |xs| EvalResult::Expr(Expr::fnum(subtract(&xs))),
    )
}

fn mul_vals(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    if vals.is_empty() {
        return EvalResult::Err("Must perform multiplication on at least one number".into());
    }
    let total = vals.iter()
        .map(|e| match eval(e.clone(), env) {
            EvalResult::Expr(exp) => match &*exp {
                Expr::FNum(n) => Ok(*n),
                _ => Err(format!("Can only sum numers, got {:?}", exp)),
            },
            _ => Err(format!("Failed to eval expr: {:?}", e)),
        }).collect::<Result<Vec<f64>, String>>();
    total.map_or_else(
        |err| EvalResult::Err(err),
        |xs| EvalResult::Expr(Expr::fnum(xs.iter().product())),
    )
}
fn divide(vals: &Vec<f64>) -> f64{
    let mut div = vals[0];
    vals[1..].iter().for_each(|x| {
        div = div / x;
    });
    div
}
fn div_vals(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    if vals.is_empty() {
        return EvalResult::Err("Must perform addition on at least one number".into());
    }
    let total = vals.iter()
        .map(|e| match eval(e.clone(), env) {
            EvalResult::Expr(exp) => match &*exp {
                Expr::FNum(n) => Ok(*n),
                _ => Err(format!("Can only sum numers, got {:?}", exp)),
            },
            _ => Err(format!("Failed to eval expr: {:?}", e)),
        }).collect::<Result<Vec<f64>, String>>();
    total.map_or_else(
        |err| EvalResult::Err(err),
        |xs| EvalResult::Expr(Expr::fnum(divide(&xs))),
    )
}
fn not(vals: &Rc<Expr>, env: &mut Environment) -> EvalResult {
    match eval(vals.clone(), env) {
            EvalResult::Err(e) => EvalResult::Err(format!("Failed to eval expr: {:?}", e)),
            _ => match &*vals.clone() {
                Expr::Symbol(s) => match &*s.as_str() {
                    "True" => return EvalResult::Expr(Expr::symbol("False".into())),
                    "False" => return EvalResult::Expr(Expr::symbol("True".into())),
                    _ => return EvalResult::Err("Invalid input for not operator 1".into()),
                },
                Expr::List(l) => EvalResult::Expr(Expr::symbol(&is_equal_list(l))) ,
                _ => EvalResult::Err("Invalid input for not operator 21".into()),
            },
            // EvalResult::Unit => match &*vals.clone() {
            //     Expr::Symbol(s) => match &*s.as_str() {
            //         "False" => EvalResult::Expr(Expr::symbol("True".into())),
            //         "True" => EvalResult::Expr(Expr::symbol("False".into())),

            //         _ => return EvalResult::Err("Invalid input for not operator 2".into()),
            //     },
            //     _ => EvalResult::Err("Not a symbol".into()),
            // },

            _ => EvalResult::Err("Failed to eval expr".into()),
    }

    
}

fn is_equal_symbol(vals: &Vec<Rc<Expr>>) -> String {
    let comparer = &vals[0];
    let mut hasFalse = false;
    let val = vals[1..].iter().for_each(|x| {
        if comparer == x {
            
        } else {
            hasFalse = true;
        }
    });
    if hasFalse {
        "True".into()
    } else {
        "False".into()
    }
}
fn is_equal_list(vals: &Vec<Rc<Expr>>) -> String {
    let comparer = &vals[0];
    let mut hasFalse: bool  = false;
    let val = vals[1..].iter().for_each(|x| {
        if comparer.eq(x)  {
            
        } else {
            hasFalse = true
        }
    });
    if hasFalse {
        "False".into()
    } else {
        "True".into()
    }
}
fn equality(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    if vals.is_empty() {
        return EvalResult::Err("'=' with no arguments".into());
    }
    let total = vals.iter()
        .map(|e| match eval(e.clone(), env) {
            //valid expression
            EvalResult::Expr(exp) => Ok(exp),
            EvalResult::Unit => Err(format!("Failed to eval, got Unit" )),
            EvalResult::Err(e) => Err(format!("Failed to eval expr: {:?}", e)),
            _ => Err("No idea whats wrong".into()),
        }).collect::<Result<Vec<Rc<Expr>>, String>>();


    total.map_or_else(
        |err| EvalResult::Err(err) ,
        |xs| EvalResult::Expr(Expr::symbol(&is_equal_list(&xs))),
    )
}
fn inequality(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    if vals.is_empty() {
        return EvalResult::Err("'not' with no arguments".into());
    }
    let total = vals.iter()
        .map(|e| match eval(e.clone(), env) {
            //valid expression
            EvalResult::Expr(exp) => Ok(exp),
            EvalResult::Unit => Err(format!("Failed to eval, got Unit" )),
            EvalResult::Err(e) => Err(format!("Failed to eval expr: {:?}", e)),
            _ => Err("No idea whats wrong".into()),
        }).collect::<Result<Vec<Rc<Expr>>, String>>();

    total.map_or_else(
        |err| EvalResult::Err(err),
        |xs| EvalResult::Expr(Expr::symbol(&is_equal_symbol(&xs))),
    )
}

fn bool_and(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    match vals.iter().all(|x| *x == vals[0]) {
        true => EvalResult::Expr(Expr::symbol("True".into())),
        false => EvalResult::Expr(Expr::symbol("False".into())),
    }
}
fn bool_or(vals: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    match vals.iter().any(|x| *x == Expr::symbol("True")) {
        true => EvalResult::Expr(Expr::symbol("True".into())),
        false => EvalResult::Expr(Expr::symbol("False".into())),
    }

}
fn if_then_else(blocks: &[Rc<Expr>], env: &mut Environment) -> EvalResult {
    if blocks.len() != 3 {
        return EvalResult::Err("Must have format: if (<argument>) (<then block>) (<else block>)".into())
    }
    match eval(blocks[0].clone(), env) {
        EvalResult::Expr(expr) => {
            match &*expr {
                Expr::List(l) if l.len() == 0 => eval(blocks[2].clone(), env),
                _ => eval(blocks[1].clone(), env),
            };
            eval(blocks[1].clone(), env)
        },
        EvalResult::Unit => EvalResult::Err("If expression predicate must return an expression.".into()),
        err => err
    }
}
/// Evaluates the given expression.
pub fn eval(e: Rc<Expr>, env: &mut Environment) -> EvalResult {
    match &*e{
        Expr::FNum(_) => EvalResult::Expr(e.clone()),
        Expr::Symbol(s) => eval_symbol(e.clone(), s, &[], env),
        Expr::List(vals) => {
            if vals.is_empty() {
                return EvalResult::Expr(Expr::list(&[]));
            }
            let op = &*vals[0];
            match op {
                Expr::Symbol(s) if s == "+" => add_vals(&vals[1..], env),
                Expr::Symbol(s) if s == "-" => sub_vals(&vals[1..], env),
                Expr::Symbol(s) if s == "*" => mul_vals(&vals[1..], env),
                Expr::Symbol(s) if s == "/" => div_vals(&vals[1..], env),
                Expr::Symbol(s) if s == "=" => equality(&vals[1..], env),
                Expr::Symbol(s) if s == "!=" => inequality(&vals[1..], env),
                Expr::Symbol(s) if s == "not" => not(&vals[1], env),
                Expr::Symbol(s) if s == "and" => bool_and(&vals[1..], env),
                Expr::Symbol(s) if s == "or" => bool_or(&vals[1..], env),
                Expr::Symbol(s) if s == "fn" => add_fn_to_env(&vals[1..], env),
                Expr::Symbol(s) if s == "let" => add_var_to_env(&vals[1..], env),
                Expr::Symbol(s) if s == "print" => {
                    let output: Vec<String> = vals[1..]
                        .iter().cloned()
                        .map(|expr| gen_print_output(expr, env)).collect();
                    println!("{}", output.join(" "));
                    EvalResult::Unit
                }
                Expr::Symbol(s) if s == "if" => if_then_else(&vals[1..], env),
                Expr::Symbol(s) if env.contains_key(&s) => eval_symbol(e.clone(), s, &vals[1..], env),
                _ => {
                    let res: Result<Vec<Rc<Expr>>, EvalResult> = vals.iter().cloned()
                    .map(|x| eval(x, env))
                    .filter(|x| *x != EvalResult::Unit)
                    .map(|x| if let EvalResult::Expr(expr) = x {
                        Ok(expr)
                    } else {
                        Err(x)
                    }).collect();

                    res.map_or_else(
                        |err| err,
                        |expr| EvalResult::Expr(Expr::list(&expr))
                    )
                }
            }
        },

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_add_to_contextless_env(){
        let mut env = Environment::empty();
        let r = env.add_var("a", Expr::fnum(1.0));
        assert!( r.is_err(), format!("Expected add_var to fail, but it succeeded: {:?}", r) );
    }
    #[test]
    fn can_add_var_to_context_env() {
        let mut env = Environment::empty();
        let val = Expr::fnum(1.0);
        assert_eq!(0usize, env.num_contexts());
        env.push_context();
        assert_eq!(1usize, env.num_contexts());

        env.add_var("a", Expr::fnum(1.0));

        env.lookup("a").map_or_else(
            || assert!(false, "Failed to find var in environment."),
            |(param, x)| {
                assert_eq!(val, x);
                assert_eq!(0usize, param.len());
            }
        );

        env.pop_context();
        env.lookup("a").map(|x| assert!(false, format!("Expected Err, got {:?}", x)));
        assert_eq!(0usize, env.num_contexts());
    }

    #[test]
    fn default_environment_is_correct() {
        let env = Environment::default();
        env.lookup("False").map_or_else(
            || assert!(false, "Expected Some, got None 1"),
            |(ps, expr)| {
                assert_eq!(0, ps.len());
                assert_eq!(Expr::list(&[]), expr);
            },
        );
        env.lookup("True").map_or_else(
            || assert!(false, "Expected Some, got None 2"),
            |(ps, expr)| {
                assert_eq!(0, ps.len());
                assert_eq!(Expr::list(&[Expr::fnum(1.0)]), expr);
            },
        );
    }

}
