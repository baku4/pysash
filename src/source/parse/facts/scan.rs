use ruff_python_ast::visitor::{Visitor, walk_expr, walk_pattern, walk_stmt};
use ruff_python_ast::{ExceptHandler, Expr, Pattern, Stmt};

/// A scope-insensitive scan of all mentioned names and reflective syntax.
#[derive(Default)]
pub struct MentionScan {
    pub mentions: Vec<String>,
    pub bare_calls: Vec<String>,
    pub method_calls: Vec<String>,
    pub opaque: bool,
}

impl MentionScan {
    pub fn run(stmt: &Stmt) -> MentionScan {
        let mut scan = MentionScan::default();
        scan.visit_stmt(stmt);
        scan
    }

    fn mention(&mut self, name: &str) {
        if reflective_name(name) {
            self.opaque = true;
        }
        if !self.mentions.iter().any(|existing| existing == name) {
            self.mentions.push(name.to_string());
        }
    }
}

impl<'a> Visitor<'a> for MentionScan {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Import(import) => {
                for alias in &import.names {
                    self.mention(alias.name.as_str());
                    if let Some(asname) = &alias.asname {
                        self.mention(asname.as_str());
                    }
                }
            }
            Stmt::ImportFrom(import) => {
                if let Some(module) = &import.module {
                    self.mention(module.as_str());
                }
                for alias in &import.names {
                    if alias.name.as_str() == "*" {
                        self.opaque = true;
                    }
                    self.mention(alias.name.as_str());
                    if let Some(asname) = &alias.asname {
                        self.mention(asname.as_str());
                    }
                }
            }
            Stmt::FunctionDef(def) => {
                self.mention(def.name.as_str());
                walk_stmt(self, stmt);
            }
            Stmt::ClassDef(class) => {
                self.mention(class.name.as_str());
                walk_stmt(self, stmt);
            }
            Stmt::Global(global) => {
                for name in &global.names {
                    self.mention(name.as_str());
                }
            }
            Stmt::Nonlocal(nonlocal) => {
                for name in &nonlocal.names {
                    self.mention(name.as_str());
                }
            }
            _ => walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => self.mention(name.id.as_str()),
            Expr::Attribute(attribute) => {
                let attr = attribute.attr.as_str();
                if reflective_attr(attr) {
                    self.opaque = true;
                }
                if attr == "modules"
                    && matches!(&*attribute.value, Expr::Name(base) if base.id.as_str() == "sys")
                {
                    self.opaque = true;
                }
                walk_expr(self, expr);
            }
            Expr::Call(call) => {
                match &*call.func {
                    // Dynamic `getattr` names are reflective; literal attribute names are bounded.
                    Expr::Name(func) if func.id.as_str() == "getattr" => {
                        self.mention("getattr");
                        self.bare_calls.push("getattr".to_string());
                        let literal_attr = call
                            .arguments
                            .args
                            .get(1)
                            .is_some_and(|arg| matches!(arg, Expr::StringLiteral(_)));
                        if !literal_attr {
                            self.opaque = true;
                        }
                    }
                    Expr::Name(func) => {
                        self.bare_calls.push(func.id.to_string());
                        self.visit_expr(&call.func);
                    }
                    Expr::Attribute(func) => {
                        self.method_calls.push(func.attr.to_string());
                        self.visit_expr(&call.func);
                    }
                    func => self.visit_expr(func),
                }
                for arg in &call.arguments.args {
                    self.visit_expr(arg);
                }
                for keyword in &call.arguments.keywords {
                    self.visit_expr(&keyword.value);
                }
            }
            _ => walk_expr(self, expr),
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ExceptHandler) {
        let ExceptHandler::ExceptHandler(inner) = handler;
        if let Some(name) = &inner.name {
            self.mention(name.as_str());
        }
        if let Some(type_) = &inner.type_ {
            self.visit_expr(type_);
        }
        for stmt in &inner.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        match pattern {
            Pattern::MatchAs(as_pattern) => {
                if let Some(name) = &as_pattern.name {
                    self.mention(name.as_str());
                }
            }
            Pattern::MatchStar(star) => {
                if let Some(name) = &star.name {
                    self.mention(name.as_str());
                }
            }
            Pattern::MatchMapping(mapping) => {
                if let Some(rest) = &mapping.rest {
                    self.mention(rest.as_str());
                }
            }
            _ => {}
        }
        walk_pattern(self, pattern);
    }
}

/// Returns whether a name can reflectively alter arbitrary module globals.
fn reflective_name(name: &str) -> bool {
    matches!(
        name,
        "__import__"
            | "builtins"
            | "delattr"
            | "eval"
            | "exec"
            | "globals"
            | "importlib"
            | "locals"
            | "setattr"
            | "vars"
    )
}

/// Returns whether accessing an attribute exposes reflective state.
fn reflective_attr(attr: &str) -> bool {
    matches!(attr, "__builtins__" | "__dict__" | "__globals__")
}
