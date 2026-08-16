use ruff_python_ast::visitor::{Visitor, walk_expr, walk_pattern, walk_stmt};
// module-rule: allow import-alias
use ruff_python_ast as ast;
use crate::statement_facts::CalleeSummary;
use super::scan::MentionScan;

/// Walks code that executes immediately and summarizes deferred callable bodies.
#[derive(Default)]
pub struct ExecWalker {
    pub sink: Sink,
    /// Local names hidden by active comprehensions.
    pub shield: Vec<String>,
}

/// Raw facts collected from the currently executing scope.
#[derive(Default)]
pub struct Sink {
    pub binds: Vec<String>,
    pub reads: Vec<String>,
    pub calls: Vec<String>,
    pub mutates: Vec<String>,
    pub deletes: Vec<String>,
    pub alias_edges: Vec<(String, String)>,
    pub global_decls: Vec<String>,
    /// Summaries for callables bound to names.
    pub nested: Vec<CalleeSummary>,
    /// Summaries for unbound callables such as argument lambdas.
    pub loose: Vec<CalleeSummary>,
    pub opaque: bool,
}

impl ExecWalker {
    pub fn run(stmt: &ast::Stmt) -> Sink {
        let mut walker = ExecWalker::default();
        walker.visit_stmt(stmt);
        walker.sink
    }

    fn shielded(&self, name: &str) -> bool {
        self.shield.iter().any(|shielded| shielded == name)
    }

    fn bind(&mut self, name: &str) {
        if !self.shielded(name) {
            push_unique(&mut self.sink.binds, name);
        }
    }

    fn read(&mut self, name: &str) {
        if !self.shielded(name) {
            push_unique(&mut self.sink.reads, name);
        }
    }

    fn mutate(&mut self, expr: &ast::Expr) {
        if let Some(root) = root_name(expr) {
            push_unique(&mut self.sink.mutates, root);
        }
    }

    fn visit_parameter_declarations(&mut self, parameters: &ast::Parameters) {
        // Defaults and annotations are evaluated when the definition executes.
        for parameter in parameters.iter() {
            if let Some(default) = parameter.default() {
                self.visit_expr(default);
            }
            if let Some(annotation) = parameter.annotation() {
                self.visit_expr(annotation);
            }
        }
    }

    fn visit_decorators(&mut self, decorators: &[ast::Decorator]) {
        for decorator in decorators {
            // A bare decorator is called at definition time; call expressions are visited below.
            if let ast::Expr::Name(name) = &decorator.expression {
                push_unique(&mut self.sink.calls, name.id.as_str());
            }
            self.visit_expr(&decorator.expression);
        }
    }
}

impl<'a> Visitor<'a> for ExecWalker {
    fn visit_stmt(&mut self, stmt: &'a ast::Stmt) {
        match stmt {
            ast::Stmt::FunctionDef(def) => {
                self.bind(def.name.as_str());
                self.visit_decorators(&def.decorator_list);
                self.visit_parameter_declarations(&def.parameters);
                if let Some(returns) = &def.returns {
                    self.visit_expr(returns);
                }
                let (summary, frees) = analyze_stmts(Some(&def.parameters), &def.body);
                for name in &frees {
                    self.read(name);
                }
                self.sink.nested.push(summary);
            }
            ast::Stmt::ClassDef(class) => {
                self.bind(class.name.as_str());
                self.visit_decorators(&class.decorator_list);
                if let Some(arguments) = &class.arguments {
                    for base in &arguments.args {
                        // Inheritance aliases mutable attributes shared with the base class.
                        if let ast::Expr::Name(base_name) = base {
                            self.sink
                                .alias_edges
                                .push((class.name.to_string(), base_name.id.to_string()));
                        }
                        self.visit_expr(base);
                    }
                    for keyword in &arguments.keywords {
                        self.visit_expr(&keyword.value);
                    }
                }
                // Class bodies execute now, but their ordinary bindings are class attributes.
                let mut inner = ExecWalker::default();
                for body_stmt in &class.body {
                    inner.visit_stmt(body_stmt);
                }
                let inner = inner.sink;
                for name in &inner.reads {
                    self.read(name);
                }
                for name in &inner.calls {
                    push_unique(&mut self.sink.calls, name);
                }
                for name in &inner.mutates {
                    push_unique(&mut self.sink.mutates, name);
                }
                // Explicit `global` bindings and deletions still affect the module.
                for name in &inner.binds {
                    if inner.global_decls.contains(name) {
                        self.bind(name);
                    }
                }
                for name in &inner.deletes {
                    if inner.global_decls.contains(name) {
                        push_unique(&mut self.sink.deletes, name);
                    }
                }
                self.sink.opaque |= inner.opaque;
                // Constructing the class may invoke any summarized method.
                let mut class_summary = CalleeSummary::default();
                for nested in inner.nested.iter().chain(&inner.loose) {
                    class_summary.absorb(nested);
                }
                self.sink.nested.push(class_summary);
            }
            ast::Stmt::Assign(assign) => {
                self.visit_expr(&assign.value);
                if let [ast::Expr::Name(target)] = &assign.targets[..] {
                    // Bare-name assignment creates a tracked alias edge.
                    if let ast::Expr::Name(value) = &*assign.value {
                        self.sink
                            .alias_edges
                            .push((target.id.to_string(), value.id.to_string()));
                    }
                    // Assignment turns the loose lambda summary into a named one.
                    if matches!(&*assign.value, ast::Expr::Lambda(_))
                        && let Some(summary) = self.sink.loose.pop()
                    {
                        self.sink.nested.push(summary);
                    }
                }
                for target in &assign.targets {
                    self.visit_expr(target);
                }
            }
            ast::Stmt::AugAssign(assign) => {
                self.visit_expr(&assign.value);
                match &*assign.target {
                    ast::Expr::Name(name) => {
                        let id = name.id.as_str();
                        self.read(id);
                        self.bind(id);
                        // Augmented assignment may mutate the existing object in place.
                        push_unique(&mut self.sink.mutates, id);
                    }
                    target => self.visit_expr(target),
                }
            }
            ast::Stmt::For(for_stmt) => {
                self.visit_expr(&for_stmt.iter);
                // Iterating may consume a generator object.
                self.mutate(&for_stmt.iter);
                self.visit_expr(&for_stmt.target);
                for body_stmt in &for_stmt.body {
                    self.visit_stmt(body_stmt);
                }
                for body_stmt in &for_stmt.orelse {
                    self.visit_stmt(body_stmt);
                }
            }
            ast::Stmt::Import(import) => {
                for alias in &import.names {
                    match &alias.asname {
                        Some(asname) => self.bind(asname.as_str()),
                        // `import a.b.c` binds the root name `a`.
                        None => {
                            let root = alias.name.split('.').next().unwrap_or_default();
                            self.bind(root);
                        }
                    }
                }
            }
            ast::Stmt::ImportFrom(import) => {
                for alias in &import.names {
                    if alias.name.as_str() == "*" {
                        self.sink.opaque = true;
                        continue;
                    }
                    match &alias.asname {
                        Some(asname) => self.bind(asname.as_str()),
                        None => self.bind(alias.name.as_str()),
                    }
                }
            }
            ast::Stmt::Global(global) => {
                for name in &global.names {
                    push_unique(&mut self.sink.global_decls, name.as_str());
                }
            }
            _ => walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a ast::Expr) {
        match expr {
            ast::Expr::Name(name) => {
                let id = name.id.as_str();
                match name.ctx {
                    ast::ExprContext::Load => self.read(id),
                    ast::ExprContext::Store => self.bind(id),
                    ast::ExprContext::Del => {
                        if !self.shielded(id) {
                            push_unique(&mut self.sink.deletes, id);
                        }
                    }
                    ast::ExprContext::Invalid => {}
                }
            }
            ast::Expr::Attribute(attribute) => {
                if matches!(
                    attribute.ctx,
                    ast::ExprContext::Store | ast::ExprContext::Del
                ) {
                    self.mutate(&attribute.value);
                }
                walk_expr(self, expr);
            }
            ast::Expr::Subscript(subscript) => {
                if matches!(
                    subscript.ctx,
                    ast::ExprContext::Store | ast::ExprContext::Del
                ) {
                    self.mutate(&subscript.value);
                }
                walk_expr(self, expr);
            }
            ast::Expr::Call(call) => {
                let pure = match &*call.func {
                    ast::Expr::Name(func) => {
                        let id = func.id.as_str();
                        push_unique(&mut self.sink.calls, id);
                        pure_call(id)
                    }
                    ast::Expr::Attribute(func) => {
                        // A method call may mutate its receiver.
                        self.mutate(&func.value);
                        false
                    }
                    _ => false,
                };
                self.visit_expr(&call.func);
                for arg in &call.arguments.args {
                    self.visit_expr(arg);
                    if !pure {
                        // An unknown call may mutate any passed argument.
                        let inner = match arg {
                            ast::Expr::Starred(starred) => &*starred.value,
                            other => other,
                        };
                        self.mutate(inner);
                    }
                }
                for keyword in &call.arguments.keywords {
                    self.visit_expr(&keyword.value);
                    if !pure {
                        self.mutate(&keyword.value);
                    }
                }
            }
            ast::Expr::Lambda(lambda) => {
                if let Some(parameters) = &lambda.parameters {
                    self.visit_parameter_declarations(parameters);
                }
                let (summary, frees) =
                    analyze_expr(lambda.parameters.as_deref(), &lambda.body);
                for name in &frees {
                    self.read(name);
                }
                self.sink.loose.push(summary);
            }
            ast::Expr::ListComp(_)
            | ast::Expr::SetComp(_)
            | ast::Expr::DictComp(_)
            | ast::Expr::Generator(_) => {
                let mark = self.shield.len();
                let generators = match expr {
                    ast::Expr::ListComp(comp) => &comp.generators,
                    ast::Expr::SetComp(comp) => &comp.generators,
                    ast::Expr::DictComp(comp) => &comp.generators,
                    ast::Expr::Generator(comp) => &comp.generators,
                    _ => unreachable!(),
                };
                for generator in generators {
                    collect_names(&generator.target, &mut self.shield);
                }
                walk_expr(self, expr);
                self.shield.truncate(mark);
            }
            _ => walk_expr(self, expr),
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(inner) = handler;
        if let Some(name) = &inner.name {
            // An exception target is bound and then deleted when the handler ends.
            self.bind(name.as_str());
            push_unique(&mut self.sink.deletes, name.as_str());
        }
        if let Some(type_) = &inner.type_ {
            self.visit_expr(type_);
        }
        for stmt in &inner.body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_pattern(&mut self, pattern: &'a ast::Pattern) {
        for name in pattern_binds(pattern) {
            self.bind(name);
        }
        walk_pattern(self, pattern);
    }
}

/// Summarizes a deferred function body and returns its free reads.
fn analyze_stmts(
    parameters: Option<&ast::Parameters>,
    body: &[ast::Stmt],
) -> (CalleeSummary, Vec<String>) {
    let mut walker = ExecWalker::default();
    let mut scan = MentionScan::default();
    for stmt in body {
        walker.visit_stmt(stmt);
        scan.visit_stmt(stmt);
    }
    finish_callable(walker.sink, scan.opaque, parameters)
}

/// Summarizes a deferred lambda body and returns its free reads.
fn analyze_expr(
    parameters: Option<&ast::Parameters>,
    body: &ast::Expr,
) -> (CalleeSummary, Vec<String>) {
    let mut walker = ExecWalker::default();
    let mut scan = MentionScan::default();
    walker.visit_expr(body);
    scan.visit_expr(body);
    finish_callable(walker.sink, scan.opaque, parameters)
}

fn finish_callable(
    mut sink: Sink,
    body_reflective: bool,
    parameters: Option<&ast::Parameters>,
) -> (CalleeSummary, Vec<String>) {
    // Reflective syntax in the body makes the call opaque.
    sink.opaque |= body_reflective;

    let positional: Vec<&str> = parameters
        .map(|parameters| {
            parameters
                .posonlyargs
                .iter()
                .chain(&parameters.args)
                .map(|parameter| parameter.parameter.name.as_str())
                .collect()
        })
        .unwrap_or_default();
    let mut all_params: Vec<&str> = positional.clone();
    if let Some(parameters) = parameters {
        for parameter in &parameters.kwonlyargs {
            all_params.push(parameter.parameter.name.as_str());
        }
        if let Some(vararg) = &parameters.vararg {
            all_params.push(vararg.name.as_str());
        }
        if let Some(kwarg) = &parameters.kwarg {
            all_params.push(kwarg.name.as_str());
        }
    }

    // A name assigned anywhere in a function is local unless declared global.
    let locals: Vec<&str> = all_params
        .iter()
        .copied()
        .chain(
            sink.binds
                .iter()
                .map(String::as_str)
                .filter(|name| !sink.global_decls.iter().any(|global| global == name)),
        )
        .collect();
    let is_local = |name: &str| locals.contains(&name);

    let mut summary = CalleeSummary {
        callees: sink.calls.iter().map(|name| name.as_str().into()).collect(),
        opaque: sink.opaque,
        ..CalleeSummary::default()
    };
    for name in sink.binds.iter().chain(&sink.deletes) {
        if sink.global_decls.iter().any(|global| global == name) {
            push_unique_boxed(&mut summary.global_writes, name);
        }
    }
    for name in &sink.mutates {
        if let Some(position) = positional.iter().position(|parameter| parameter == name) {
            if !summary.mutates_params.contains(&position) {
                summary.mutates_params.push(position);
            }
        } else if !is_local(name) {
            push_unique_boxed(&mut summary.mutates_frees, name);
        }
    }
    // A call can also trigger effects from callables defined inside the body.
    for nested in sink.nested.iter().chain(&sink.loose) {
        for name in &nested.global_writes {
            push_unique_boxed(&mut summary.global_writes, name);
        }
        for name in &nested.mutates_frees {
            if !is_local(name) {
                push_unique_boxed(&mut summary.mutates_frees, name);
            }
        }
        for name in &nested.callees {
            if !is_local(name) {
                push_unique_boxed(&mut summary.callees, name);
            }
        }
        summary.opaque |= nested.opaque;
    }

    let frees = sink
        .reads
        .iter()
        .filter(|name| !is_local(name))
        .cloned()
        .collect();
    (summary, frees)
}

/// Returns whether a builtin is assumed not to mutate its arguments.
fn pure_call(name: &str) -> bool {
    matches!(
        name,
        "abs" | "all"
            | "any"
            | "bool"
            | "callable"
            | "chr"
            | "dict"
            | "divmod"
            | "enumerate"
            | "float"
            | "format"
            | "frozenset"
            | "hash"
            | "hex"
            | "id"
            | "int"
            | "isinstance"
            | "issubclass"
            | "iter"
            | "len"
            | "list"
            | "map"
            | "max"
            | "min"
            | "oct"
            | "ord"
            | "print"
            | "range"
            | "repr"
            | "reversed"
            | "round"
            | "set"
            | "sorted"
            | "str"
            | "sum"
            | "tuple"
            | "type"
            | "zip"
    )
}

/// Returns the root name of an access chain such as `x.a.b[k]`.
fn root_name(expr: &ast::Expr) -> Option<&str> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.as_str()),
        ast::Expr::Attribute(attribute) => root_name(&attribute.value),
        ast::Expr::Subscript(subscript) => root_name(&subscript.value),
        ast::Expr::Starred(starred) => root_name(&starred.value),
        _ => None,
    }
}

/// Collects every name in an assignment target, including unpacking.
fn collect_names(expr: &ast::Expr, out: &mut Vec<String>) {
    match expr {
        ast::Expr::Name(name) => out.push(name.id.to_string()),
        ast::Expr::Tuple(tuple) => {
            for element in &tuple.elts {
                collect_names(element, out);
            }
        }
        ast::Expr::List(list) => {
            for element in &list.elts {
                collect_names(element, out);
            }
        }
        ast::Expr::Starred(starred) => collect_names(&starred.value, out),
        _ => {}
    }
}

/// Returns names bound by a match pattern.
fn pattern_binds(pattern: &ast::Pattern) -> Vec<&str> {
    let mut names = Vec::new();
    fn walk<'a>(pattern: &'a ast::Pattern, out: &mut Vec<&'a str>) {
        match pattern {
            ast::Pattern::MatchAs(as_pattern) => {
                if let Some(name) = &as_pattern.name {
                    out.push(name.as_str());
                }
                if let Some(inner) = &as_pattern.pattern {
                    walk(inner, out);
                }
            }
            ast::Pattern::MatchStar(star) => {
                if let Some(name) = &star.name {
                    out.push(name.as_str());
                }
            }
            ast::Pattern::MatchMapping(mapping) => {
                if let Some(rest) = &mapping.rest {
                    out.push(rest.as_str());
                }
                for inner in &mapping.patterns {
                    walk(inner, out);
                }
            }
            ast::Pattern::MatchSequence(sequence) => {
                for inner in &sequence.patterns {
                    walk(inner, out);
                }
            }
            ast::Pattern::MatchOr(or_pattern) => {
                for inner in &or_pattern.patterns {
                    walk(inner, out);
                }
            }
            ast::Pattern::MatchClass(class) => {
                for inner in &class.arguments.patterns {
                    walk(inner, out);
                }
                for keyword in &class.arguments.keywords {
                    walk(&keyword.pattern, out);
                }
            }
            ast::Pattern::MatchValue(_) | ast::Pattern::MatchSingleton(_) => {}
        }
    }
    walk(pattern, &mut names);
    names
}

fn push_unique(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|existing| existing == name) {
        names.push(name.to_string());
    }
}

fn push_unique_boxed(names: &mut Vec<Box<str>>, name: &str) {
    if !names.iter().any(|existing| &**existing == name) {
        names.push(name.into());
    }
}
