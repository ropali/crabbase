#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Literal(String),
    Operator(String),
    And,
    Or,
    LParen,
    RParen,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Binary {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Logical {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Variable(String),
    Value(String),
}

/// Tokenize the raw filter query string
pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = vec![];
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                chars.next();
            }
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            '&' => {
                tokens.push(Token::And);
                chars.next();
            }
            '|' => {
                tokens.push(Token::Or);
                chars.next();
            }
            '=' | '!' | '<' | '>' | '~' => {
                let mut op = String::new();
                op.push(chars.next().unwrap());

                if let Some(&next_c) = chars.peek()
                    && (next_c == '=' || next_c == '~')
                {
                    op.push(chars.next().unwrap());
                }

                tokens.push(Token::Operator(op));
            }
            '\'' | '"' => {
                let quote = chars.next().unwrap();
                let mut literal = String::new();

                while let Some(&next_c) = chars.peek() {
                    if next_c == quote {
                        chars.next();
                        break;
                    }
                    literal.push(chars.next().unwrap());
                }
                tokens.push(Token::Literal(literal));
            }

            _ => {
                let mut indent = String::new();

                while let Some(&next_c) = chars.peek() {
                    if next_c.is_alphanumeric() || next_c == '_' || next_c == '.' || next_c == '@' {
                        indent.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                if !indent.is_empty() {
                    tokens.push(Token::Ident(indent));
                } else {
                    chars.next();
                }
            }
        }
    }

    tokens
}

pub struct RuleParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl RuleParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Expr, String> {
        self.parse_logical()
    }

    fn parse_logical(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;

        while let Some(token) = self.peek() {
            match token {
                Token::And => {
                    self.consume();
                    let right = self.parse_comparison()?;

                    left = Expr::Logical {
                        left: Box::new(left),
                        op: "AND".to_string(),
                        right: Box::new(right),
                    };
                }
                Token::Or => {
                    self.consume();
                    let right = self.parse_comparison()?;

                    left = Expr::Logical {
                        left: Box::new(left),
                        op: "OR".to_string(),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let left = self.parse_primary()?;

        if let Some(Token::Operator(op)) = self.peek() {
            let op_str = op.clone();
            self.consume();

            let right = self.parse_primary()?;

            return Ok(Expr::Binary {
                left: Box::new(left),
                op: op_str,
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::LParen) => {
                self.consume();
                let expr = self.parse()?;

                if self.peek() != Some(&Token::RParen) {
                    return Err("Expected matching close parenthisis".to_string());
                }

                self.consume();

                Ok(expr)
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.consume();
                Ok(Expr::Variable(name))
            }
            Some(Token::Literal(val)) => {
                let val = val.clone();
                self.consume();
                Ok(Expr::Value(val))
            }
            _ => Err("Invalid syntax in rule expression".to_string()),
        }
    }

    fn consume(&mut self) {
        self.pos += 1;
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_helper(input: &str) -> Result<Expr, String> {
        let tokens = tokenize(input);
        RuleParser::new(tokens).parse()
    }

    #[test]
    fn test_parse_simple_comparison() {
        let ast = parse_helper("status = 'active'").unwrap();
        match ast {
            Expr::Binary { left, op, right } => {
                assert_eq!(*left, Expr::Variable("status".to_string()));
                assert_eq!(op, "=");
                assert_eq!(*right, Expr::Value("active".to_string()));
            }
            _ => panic!("Expected Binary expression"),
        }
    }

    #[test]
    fn test_parse_logical_and() {
        let ast = parse_helper("status = 'active' & age > '18'").unwrap();
        if let Expr::Logical { left, op, right } = ast {
            assert_eq!(op, "AND");
            if let Expr::Binary {
                left: l_left,
                op: l_op,
                right: l_right,
            } = *left
            {
                assert_eq!(*l_left, Expr::Variable("status".to_string()));
                assert_eq!(l_op, "=");
                assert_eq!(*l_right, Expr::Value("active".to_string()));
            } else {
                panic!("Expected left arm of AND to be Binary");
            }
            if let Expr::Binary {
                left: r_left,
                op: r_op,
                right: r_right,
            } = *right
            {
                assert_eq!(*r_left, Expr::Variable("age".to_string()));
                assert_eq!(r_op, ">");
                assert_eq!(*r_right, Expr::Value("18".to_string()));
            } else {
                panic!("Expected right arm of AND to be Binary");
            }
        } else {
            panic!("Expected Logical expression");
        }
    }

    #[test]
    fn test_parse_logical_or() {
        let ast = parse_helper("role = 'admin' | role = 'editor'").unwrap();
        if let Expr::Logical { left, op, right } = ast {
            assert_eq!(op, "OR");
            assert!(matches!(*left, Expr::Binary { .. }));
            assert!(matches!(*right, Expr::Binary { .. }));
        } else {
            panic!("Expected Logical OR expression");
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let ast = parse_helper("(status = 'active')").unwrap();
        assert!(matches!(ast, Expr::Binary { .. }));
    }

    #[test]
    fn test_parse_complex_precedence() {
        let ast = parse_helper("status = 'active' & (role = 'admin' | role = 'editor')").unwrap();
        if let Expr::Logical { left, op, right } = ast {
            assert_eq!(op, "AND");
            assert!(matches!(*left, Expr::Binary { .. }));
            if let Expr::Logical {
                left: r_l,
                op: r_op,
                right: r_r,
            } = *right
            {
                assert_eq!(r_op, "OR");
                assert!(matches!(*r_l, Expr::Binary { .. }));
                assert!(matches!(*r_r, Expr::Binary { .. }));
            } else {
                panic!("Expected right side of AND to be Logical OR");
            }
        } else {
            panic!("Expected outer Logical AND expression");
        }
    }

    #[test]
    fn test_parse_errors() {
        let err = parse_helper("(status = 'active'").unwrap_err();
        assert!(err.contains("Expected matching close parenthisis"));

        let err2 = parse_helper("=").unwrap_err();
        assert!(err2.contains("Invalid syntax in rule expression"));
    }
}
