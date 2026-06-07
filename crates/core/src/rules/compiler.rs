use std::collections::HashMap;

use crate::rules::parser::Expr;

pub struct SqlContext {
    pub auth: Option<HashMap<String, String>>, // Maps "@request.auth.id" etc
    pub query: HashMap<String, String>,        // Maps "@request,query.x" etc
}

pub struct RulesSqlCompiler {
    context: SqlContext,
    pub bindings: Vec<String>,
}

impl RulesSqlCompiler {
    pub fn new(context: SqlContext) -> Self {
        Self {
            context,
            bindings: Vec::new(),
        }
    }

    pub fn compile(&mut self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Variable(name) => {
                if name.starts_with("@request.auth.") {
                    let key = name.strip_prefix("@request.auth.").unwrap();
                    let val = self
                        .context
                        .auth
                        .as_ref()
                        .and_then(|m| m.get(key))
                        .cloned()
                        .unwrap_or_default();

                    self.bindings.push(val);

                    Ok("?".to_uppercase())
                } else if name.starts_with("@request.query.") {
                    let key = name.strip_prefix("@request.query.").unwrap();

                    let val = self.context.query.get(key).cloned().unwrap_or_default();

                    self.bindings.push(val);

                    Ok("?".to_string())
                } else {
                    // Safe verification: Ensure characters are safe alphnum to prevent SQL  injection
                    if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        Ok(format!("\"{}\"", name))
                    } else {
                        Err(format!("Unsafe identifier: {}", name))
                    }
                }
            }
            Expr::Binary { left, op, right } => {
                let sql_left = self.compile(left)?;
                let sql_right = self.compile(right)?;
                let sql_op = match op.as_str() {
                    "=" => "=",
                    "!=" => "!=",
                    "<" => "<",
                    ">" => ">",
                    "<=" => "<=",
                    ">=" => ">=",
                    "~" => "LIKE",
                    _ => return Err(format!("Unsupported operator: {}", op)),
                };
                Ok(format!("({} {} {})", sql_left, sql_op, sql_right))
            }
            Expr::Logical { left, op, right } => {
                let sql_left = self.compile(left)?;
                let sql_right = self.compile(right)?;
                Ok(format!("({} {} {})", sql_left, op, sql_right))
            }
            Expr::Value(val) => {
                self.bindings.push(val.clone());
                Ok("?".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile_helper(input: &str, context: SqlContext) -> Result<(String, Vec<String>), String> {
        let tokens = crate::rules::parser::tokenize(input);
        let expr = crate::rules::parser::RuleParser::new(tokens).parse()?;
        let mut compiler = RulesSqlCompiler::new(context);
        let sql = compiler.compile(&expr)?;
        Ok((sql, compiler.bindings))
    }

    #[test]
    fn test_compile_simple_comparison() {
        let context = SqlContext {
            auth: None,
            query: HashMap::new(),
        };
        let (sql, bindings) = compile_helper("status = 'active'", context).unwrap();
        assert_eq!(sql, "(\"status\" = ?)");
        assert_eq!(bindings, vec!["active".to_string()]);
    }

    #[test]
    fn test_compile_operators() {
        let operators = vec![
            ("status = 'active'", "(\"status\" = ?)"),
            ("status != 'active'", "(\"status\" != ?)"),
            ("age < '18'", "(\"age\" < ?)"),
            ("age > '18'", "(\"age\" > ?)"),
            ("age <= '18'", "(\"age\" <= ?)"),
            ("age >= '18'", "(\"age\" >= ?)"),
            ("name ~ 'admin'", "(\"name\" LIKE ?)"),
        ];

        for (input, expected_sql) in operators {
            let context = SqlContext {
                auth: None,
                query: HashMap::new(),
            };
            let (sql, bindings) = compile_helper(input, context).unwrap();
            assert_eq!(sql, expected_sql);
            assert_eq!(bindings.len(), 1);
        }
    }

    #[test]
    fn test_compile_logical_expressions() {
        let context = SqlContext {
            auth: None,
            query: HashMap::new(),
        };
        let (sql, bindings) = compile_helper("status = 'active' & role = 'admin'", context).unwrap();
        assert_eq!(sql, "((\"status\" = ?) AND (\"role\" = ?))");
        assert_eq!(bindings, vec!["active".to_string(), "admin".to_string()]);
    }

    #[test]
    fn test_compile_auth_context() {
        let mut auth = HashMap::new();
        auth.insert("id".to_string(), "user_123".to_string());
        auth.insert("role".to_string(), "admin".to_string());

        let context = SqlContext {
            auth: Some(auth),
            query: HashMap::new(),
        };

        let (sql, bindings) = compile_helper("owner_id = @request.auth.id", context).unwrap();
        assert_eq!(sql, "(\"owner_id\" = ?)");
        assert_eq!(bindings, vec!["user_123".to_string()]);
    }

    #[test]
    fn test_compile_query_context() {
        let mut query = HashMap::new();
        query.insert("search".to_string(), "rust".to_string());

        let context = SqlContext {
            auth: None,
            query,
        };

        let (sql, bindings) = compile_helper("title ~ @request.query.search", context).unwrap();
        assert_eq!(sql, "(\"title\" LIKE ?)");
        assert_eq!(bindings, vec!["rust".to_string()]);
    }

    #[test]
    fn test_compile_missing_context_vars() {
        let context = SqlContext {
            auth: None,
            query: HashMap::new(),
        };

        let (sql, bindings) = compile_helper("owner_id = @request.auth.id", context).unwrap();
        assert_eq!(sql, "(\"owner_id\" = ?)");
        assert_eq!(bindings, vec!["".to_string()]);
    }

    #[test]
    fn test_compile_unsafe_identifier() {
        let context = SqlContext {
            auth: None,
            query: HashMap::new(),
        };

        let unsafe_expr = Expr::Variable("status; DROP TABLE users;".to_string());
        let mut compiler = RulesSqlCompiler::new(context);
        let res = compiler.compile(&unsafe_expr);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Unsafe identifier"));
    }

    #[test]
    fn test_unsupported_operator() {
        let context = SqlContext {
            auth: None,
            query: HashMap::new(),
        };
        let expr = Expr::Binary {
            left: Box::new(Expr::Variable("age".to_string())),
            op: "!!".to_string(),
            right: Box::new(Expr::Value("18".to_string())),
        };
        let mut compiler = RulesSqlCompiler::new(context);
        let res = compiler.compile(&expr);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Unsupported operator: !!"));
    }
}
